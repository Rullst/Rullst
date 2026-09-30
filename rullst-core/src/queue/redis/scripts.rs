//! Lua scripts that make each Redis queue state transition atomic.

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

pub(super) const FAIL_SCRIPT: &str = r#"
local raw = redis.call('HGET', KEYS[2], ARGV[1])
if not raw then return 0 end
local expected = ARGV[3]
if expected and expected ~= '' then
    local ok, envelope = pcall(cjson.decode, raw)
    if not ok or type(envelope) ~= 'table' or envelope.attempts ~= tonumber(expected) then return 0 end
end
redis.call('ZREM', KEYS[1], raw)
redis.call('HDEL', KEYS[2], ARGV[1])
redis.call('HSET', KEYS[3], ARGV[1], cjson.encode({ raw = raw, error = ARGV[2] }))
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
return recovered
"#;

pub(super) const PENDING_COUNT_SCRIPT: &str = r#"
return redis.call('LLEN', KEYS[1]) + redis.call('ZCARD', KEYS[2])
"#;
