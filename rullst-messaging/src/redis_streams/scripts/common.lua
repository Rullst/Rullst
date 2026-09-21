-- All keys belong to one dedicated standalone namespace. Cluster is unsupported.
local meta = KEYS[1]
local prefix = string.sub(meta, 1, -5)
local log = prefix .. 'log'
local records = prefix .. 'records'
local idem = prefix .. 'idem'
local subscriptions = prefix .. 'subscriptions'
local tokens = prefix .. 'tokens'
local remaining = prefix .. 'remaining'
local signature = ARGV[1]
local max_messages = tonumber(ARGV[2])
local max_subscriptions = tonumber(ARGV[3])
local max_attempts = tonumber(ARGV[4])
local function call(...) return redis.call(...) end
local function number(value)
    local result = tonumber(value)
    if not result or result < 0 or result ~= math.floor(result) or result > 9007199254740000 then
        error('invalid numeric state')
    end
    return result
end
local function text(value) return string.format('%.0f', value) end
local function now()
    local value = call('TIME')
    return number(number(value[1]) * 1000 + math.floor(number(value[2]) / 1000))
end
local function type_of(key) return call('TYPE', key).ok end
local function topic_keys(topic)
    return prefix .. 'topic:' .. topic, prefix .. 'groups:' .. topic, prefix .. 'terminal:' .. topic
end
local function group_keys(group)
    return prefix .. 'ready:' .. group, prefix .. 'state:' .. group,
        prefix .. 'done:' .. group, prefix .. 'dead:' .. group
end
local function guard()
    if type_of(meta) ~= 'hash' then return {'missing'} end
    if call('HGET', meta, 'signature') ~= signature then return {'config'} end
    if call('HGET', meta, 'dirty') ~= '0' then return {'corrupt'} end
    for _, key in ipairs({records, idem, subscriptions, tokens, remaining}) do
        if type_of(key) ~= 'hash' or call('PTTL', key) ~= -1 then return {'corrupt'} end
        if call('HGET', key, '_') ~= signature then return {'corrupt'} end
    end
    if type_of(log) ~= 'stream' or call('PTTL', log) ~= -1 or call('PTTL', meta) ~= -1 then
        return {'corrupt'}
    end
    local count = number(call('HGET', meta, 'count'))
    if count ~= call('HLEN', records) - 1 or count ~= call('HLEN', idem) - 1
        or count ~= call('HLEN', remaining) - 1 or count ~= call('XLEN', log)
        or count > max_messages or call('HLEN', subscriptions) - 1 > max_subscriptions then
        return {'corrupt'}
    end
    if number(call('HGET', meta, 'bytes')) > 67108864 then return {'corrupt'} end
    return nil
end
local function group_guard(group)
    local stored = call('HGET', subscriptions, group)
    if not stored then return nil end
    local data = cjson.decode(stored)
    local ready, state, done, dead = group_keys(group)
    if call('ZCARD', ready) + call('ZCARD', done) ~= number(data[2])
        or call('ZCARD', dead) > call('ZCARD', done) then error('group index corrupt') end
    return data
end
local function frame(seq)
    local entries = call('XRANGE', log, seq .. '-0', seq .. '-0', 'COUNT', 1)
    if #entries ~= 1 or #entries[1][2] ~= 2 or entries[1][2][1] ~= 'wire' then
        error('envelope log corrupt')
    end
    local bytes = entries[1][2][2]
    if #bytes > 1060000 then error('envelope oversized') end
    return bytes
end
local function record(seq)
    local value = call('HGET', records, seq)
    if not value then error('message missing') end
    return cjson.decode(value)
end
local function finish_delivery(group, seq, state_data, failure, timestamp)
    local ready, state, done, dead = group_keys(group)
    if state_data[2] ~= '' then call('HDEL', tokens, state_data[2]) end
    call('ZREM', ready, seq)
    state_data[2] = ''
    state_data[3] = 0
    state_data[4] = failure
    state_data[5] = text(timestamp)
    call('HSET', state, seq, cjson.encode(state_data))
    call('ZADD', done, number(seq), seq)
    if failure ~= '' then call('ZADD', dead, number(seq), seq) end
    local pending = number(call('HGET', remaining, seq))
    if pending < 1 then error('terminal counter corrupt') end
    call('HSET', remaining, seq, pending - 1)
    if pending == 1 then
        local message = record(seq)
        local _, _, terminal = topic_keys(message[2])
        call('ZADD', terminal, number(seq), seq)
    end
end
local function mutate(operation)
    local failure = guard()
    if failure then return failure end
    local timestamp = now()
    if timestamp < number(call('HGET', meta, 'clock')) then return {'clock'} end
    -- Redis scripts isolate mutations but do not roll them back. This marker is
    -- cleared only after every command succeeds. A partial error quarantines it.
    call('HSET', meta, 'dirty', '1', 'clock', text(timestamp))
    local result = operation(timestamp)
    call('HSET', meta, 'dirty', '0')
    return result
end
