return mutate(function(timestamp)
    local token, operation, delay, failure = ARGV[5], ARGV[6], number(ARGV[7]), ARGV[8]
    local pointer = call('HGET', tokens, token)
    if not pointer then return {'lease'} end
    local binding = cjson.decode(pointer)
    local group, seq = binding[1], binding[2]
    if not group_guard(group) then error('lease group missing') end
    local ready, state = group_keys(group)
    local stored = call('HGET', state, seq)
    if not stored then error('lease state missing') end
    local data = cjson.decode(stored)
    if data[2] ~= token or number(call('ZSCORE', ready, seq)) ~= number(data[3]) then
        error('lease binding corrupt')
    end
    local expired = number(data[3]) <= timestamp
    if expired or operation == 'retry' then
        if number(data[1]) >= max_attempts then
            finish_delivery(group, seq, data, expired and 'delivery.max_attempts' or failure, timestamp)
            if expired then return {'expired'} end
            return {'ok', 'dead', '0'}
        end
        call('HDEL', tokens, token)
        data[2], data[3] = '', 0
        call('HSET', state, seq, cjson.encode(data))
        local available = timestamp + (expired and 0 or delay)
        call('ZADD', ready, available, seq)
        if expired then return {'expired'} end
        return {'ok', 'retry', text(available)}
    end
    finish_delivery(group, seq, data, operation == 'ack' and '' or failure, timestamp)
    return {'ok'}
end)
