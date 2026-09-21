return mutate(function(timestamp)
    local topic, group, start = ARGV[5], ARGV[6], ARGV[7]
    local ready, state = group_keys(group)
    local prior = group_guard(group)
    if prior then
        local count = 0
        local entries = call('ZRANGE', ready, 0, max_messages)
        if #entries > max_messages then error('pending limit corrupt') end
        for _, seq in ipairs(entries) do
            local stored = call('HGET', state, seq)
            if not stored or cjson.decode(stored)[2] == '' then count = count + 1 end
        end
        return {'ok', 0, count}
    end
    if call('HLEN', subscriptions) - 1 >= max_subscriptions then return {'subscriptions'} end
    local index, groups, terminal = topic_keys(topic)
    local initial = number(call('HGET', meta, 'sequence'))
    local pending = {}
    if start == 'earliest' then
        initial = 0
        pending = call('ZRANGE', index, 0, max_messages)
        if #pending > max_messages then error('topic capacity corrupt') end
        for _, seq in ipairs(pending) do
            local count = number(call('HGET', remaining, seq))
            call('HSET', remaining, seq, count + 1)
            call('ZREM', terminal, seq)
            call('ZADD', ready, timestamp, seq)
        end
    end
    call('SADD', groups, group)
    call('HSET', subscriptions, group, cjson.encode({text(initial), #pending}))
    return {'ok', 1, #pending}
end)
