//! Lua scripts that make each Redis queue state transition atomic.
//!
//! Failure retention is bounded: every script that appends to the dead-letter
//! list trims it to the configured count (newest kept), and the failure script
//! indexes failed jobs by server time and evicts the oldest beyond the limit.

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

pub(super) const RECOVER_SCRIPT: &str = r#"
local stalled = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1])
local recovered = 0
for _, raw in ipairs(stalled) do
    redis.call('ZREM', KEYS[1], raw)
    local ok, envelope = pcall(cjson.decode, raw)
    if ok and type(envelope) == 'table' and type(envelope.id) == 'string' then
        redis.call('HDEL', KEYS[2], envelope.id)
        redis.call('RPUSH', KEYS[3], raw)
        recovered = recovered + 1
    else
        redis.call('RPUSH', KEYS[4], cjson.encode({ raw = raw, error = 'invalid stalled job envelope' }))
    end
end
redis.call('LTRIM', KEYS[4], -tonumber(ARGV[2]), -1)
return recovered
"#;

pub(super) const PENDING_COUNT_SCRIPT: &str = r#"
return redis.call('LLEN', KEYS[1]) + redis.call('ZCARD', KEYS[2])
"#;
