//! Lua scripts that make each Redis queue state transition atomic.
//!
//! Failure retention is bounded: every script that appends to the dead-letter
//! list trims it to the configured count (newest kept), and the failure script
//! indexes failed jobs by server time and evicts the oldest beyond the limit.

// ARGV[1] is the dead-letter retention. A positive ARGV[2] records the
// claim's lease: the claim stalls only lease_ms after the server-time claim.
pub(super) const CLAIM_SCRIPT: &str = r#"
local now = redis.call('TIME')
local claimed_at_ms = (tonumber(now[1]) * 1000) + math.floor(tonumber(now[2]) / 1000)
local due = redis.call('ZRANGEBYSCORE', KEYS[5], '-inf', claimed_at_ms, 'LIMIT', 0, 100)
for _, scheduled_raw in ipairs(due) do
    if redis.call('ZREM', KEYS[5], scheduled_raw) == 1 then
        redis.call('RPUSH', KEYS[1], scheduled_raw)
    end
end
local raw = redis.call('LPOP', KEYS[1])
if not raw then return nil end
local ok, envelope = pcall(cjson.decode, raw)
if ok and type(envelope) == 'table' and type(envelope.attempts) == 'number' then
    envelope.attempts = envelope.attempts + 1
    local lease_ms = tonumber(ARGV[2]) or 0
    if lease_ms > 0 then
        envelope.lease_expires_at_ms = claimed_at_ms + lease_ms
    else
        envelope.lease_expires_at_ms = nil
    end
    raw = cjson.encode(envelope)
end
if ok and type(envelope) == 'table' and type(envelope.id) == 'string' and envelope.id ~= '' then
    if redis.call('HEXISTS', KEYS[3], envelope.id) == 1 then
        redis.call('RPUSH', KEYS[4], cjson.encode({ raw = raw, error = 'duplicate processing job id' }))
        redis.call('LTRIM', KEYS[4], -tonumber(ARGV[1]), -1)
        return redis.error_reply('duplicate processing job id')
    end
    redis.call('HSET', KEYS[3], envelope.id, raw)
end
redis.call('ZADD', KEYS[2], claimed_at_ms, raw)
return raw
"#;

pub(super) const REJECT_SCRIPT: &str = r#"
redis.call('ZREM', KEYS[1], ARGV[1])
local ok, envelope = pcall(cjson.decode, ARGV[1])
if ok and type(envelope) == 'table' and type(envelope.id) == 'string' then
    redis.call('HDEL', KEYS[2], envelope.id)
end
redis.call('RPUSH', KEYS[3], cjson.encode({ raw = ARGV[1], error = ARGV[2] }))
redis.call('LTRIM', KEYS[3], -tonumber(ARGV[3]), -1)
return 1
"#;

// Transition scripts take the job id as ARGV[1] and the claimed attempt as
// the last argument. A non-empty attempt fences the transition: it only
// applies while the processing lease still carries that attempt number, so
// a stale worker cannot finish a job that was recovered and claimed again.
pub(super) const COMPLETE_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
local expected = ARGV[2]
if expected and expected ~= '' then
    local ok, envelope = pcall(cjson.decode, raw)
    if not ok or type(envelope) ~= 'table' or envelope.attempts ~= tonumber(expected) then return 0 end
end
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
return 1
"#;

// KEYS[4] indexes failed job ids by failure time; ARGV[4] is the retention
// limit. Failures recorded before the index existed are not counted.
pub(super) const FAIL_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
local expected = ARGV[3]
if expected and expected ~= '' then
    local ok, envelope = pcall(cjson.decode, raw)
    if not ok or type(envelope) ~= 'table' or envelope.attempts ~= tonumber(expected) then return 0 end
end
local now = redis.call('TIME')
local failed_at_ms = (tonumber(now[1]) * 1000) + math.floor(tonumber(now[2]) / 1000)
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('HSET', KEYS[3], ARGV[1], cjson.encode({ raw = raw, error = ARGV[2] }))
redis.call('ZADD', KEYS[4], failed_at_ms, ARGV[1])
local excess = redis.call('ZCARD', KEYS[4]) - tonumber(ARGV[4])
if excess > 0 then
    for _, evicted in ipairs(redis.call('ZRANGE', KEYS[4], 0, excess - 1)) do
        redis.call('HDEL', KEYS[3], evicted)
    end
    redis.call('ZREMRANGEBYRANK', KEYS[4], 0, excess - 1)
end
return 1
"#;

pub(super) const REQUEUE_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
local expected = ARGV[3]
if expected and expected ~= '' then
    local ok, envelope = pcall(cjson.decode, raw)
    if not ok or type(envelope) ~= 'table' or envelope.attempts ~= tonumber(expected) then return 0 end
end
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('LPUSH', KEYS[3], raw)
return 1
"#;

// Moves a fenced processing lease into the scheduled set, due ARGV[2]
// milliseconds after the Redis server time. The claim script promotes it to the
// tail of the pending list once it is due.
pub(super) const REQUEUE_AFTER_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
local expected = ARGV[3]
if expected and expected ~= '' then
    local ok, envelope = pcall(cjson.decode, raw)
    if not ok or type(envelope) ~= 'table' or envelope.attempts ~= tonumber(expected) then return 0 end
end
local now = redis.call('TIME')
local due_ms = (tonumber(now[1]) * 1000) + math.floor(tonumber(now[2]) / 1000) + tonumber(ARGV[2])
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('ZADD', KEYS[3], due_ms, raw)
return 1
"#;

// Recovers stalled leases. A lease that recorded lease_expires_at_ms stalls
// once server time passes it; any other lease once it is at least ARGV[1]
// milliseconds old by server time (claims are scored with server time, so a
// worker host's clock skew cannot shift the cutoff). KEYS: processing set,
// processing index, pending list, dead letters, failed hash, failed index.
// ARGV[3] is the stalled-lease ceiling: the lease that reaches it fails the
// job with message ARGV[4] (failed retention ARGV[5]) instead of requeuing
// it, so a job that keeps crashing its worker cannot be reclaimed forever.
pub(super) const RECOVER_SCRIPT: &str = r#"
local now = redis.call('TIME')
local now_ms = (tonumber(now[1]) * 1000) + math.floor(tonumber(now[2]) / 1000)
local cutoff_ms = now_ms - tonumber(ARGV[1])
local leases = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', '+inf', 'WITHSCORES')
local recovered = 0
for index = 1, #leases, 2 do
    local raw = leases[index]
    local ok, envelope = pcall(cjson.decode, raw)
    local valid = ok and type(envelope) == 'table' and type(envelope.id) == 'string'
    local deadline = nil
    if valid then deadline = tonumber(envelope.lease_expires_at_ms) end
    local expired
    if deadline then
        expired = deadline <= now_ms
    else
        expired = tonumber(leases[index + 1]) <= cutoff_ms
    end
    if expired then
        redis.call('ZREM', KEYS[1], raw)
        if valid then
            redis.call('HDEL', KEYS[2], envelope.id)
            local stalls = (tonumber(envelope.stalled_recoveries) or 0) + 1
            if stalls >= tonumber(ARGV[3]) then
                redis.call('HSET', KEYS[5], envelope.id, cjson.encode({ raw = raw, error = ARGV[4] }))
                redis.call('ZADD', KEYS[6], now_ms, envelope.id)
                local excess = redis.call('ZCARD', KEYS[6]) - tonumber(ARGV[5])
                if excess > 0 then
                    for _, evicted in ipairs(redis.call('ZRANGE', KEYS[6], 0, excess - 1)) do
                        redis.call('HDEL', KEYS[5], evicted)
                    end
                    redis.call('ZREMRANGEBYRANK', KEYS[6], 0, excess - 1)
                end
            else
                envelope.stalled_recoveries = stalls
                envelope.lease_expires_at_ms = nil
                redis.call('RPUSH', KEYS[3], cjson.encode(envelope))
            end
            recovered = recovered + 1
        else
            redis.call('RPUSH', KEYS[4], cjson.encode({ raw = raw, error = 'invalid stalled job envelope' }))
        end
    end
end
redis.call('LTRIM', KEYS[4], -tonumber(ARGV[2]), -1)
return recovered
"#;

// Moves a failed job back to the tail of the pending list. The leased raw
// envelope keeps its attempt counter, so the next claim still increments it
// and older leases stay fenced.
pub(super) const RETRY_FAILED_SCRIPT: &str = r#"
local entry = redis.call('HGET', KEYS[1], ARGV[1])
if not entry then return 0 end
local ok, failure = pcall(cjson.decode, entry)
if not ok or type(failure) ~= 'table' or type(failure.raw) ~= 'string' then return -1 end
redis.call('HDEL', KEYS[1], ARGV[1])
redis.call('ZREM', KEYS[2], ARGV[1])
local raw = failure.raw
local decoded, envelope = pcall(cjson.decode, raw)
if decoded and type(envelope) == 'table' and envelope.stalled_recoveries ~= nil then
    -- A manual retry starts a fresh stalled-lease count.
    envelope.stalled_recoveries = nil
    raw = cjson.encode(envelope)
end
redis.call('RPUSH', KEYS[3], raw)
return 1
"#;

pub(super) const PENDING_COUNT_SCRIPT: &str = r#"
return redis.call('LLEN', KEYS[1]) + redis.call('ZCARD', KEYS[2])
"#;
