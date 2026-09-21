return mutate(function(timestamp)
    local operation, binding, limit = ARGV[5], ARGV[6], number(ARGV[7])
    if operation == 'dead' then
        if not group_guard(binding) then return {'subscription'} end
        local _, state, _, dead = group_keys(binding)
        local candidates = call('ZRANGE', dead, 0, limit - 1)
        local response, bytes = {'ok'}, 0
        for _, seq in ipairs(candidates) do
            local wire = frame(seq)
            if bytes + #wire > 4194304 then break end
            local data = cjson.decode(call('HGET', state, seq))
            response[#response + 1] = {wire, data[1], data[4], text(data[5])}
            bytes = bytes + #wire
        end
        return response
    end
    local index, groups, terminal = topic_keys(binding)
    local subscribers = call('SMEMBERS', groups)
    if #subscribers == 0 then return {'ok', 0} end
    if #subscribers > max_subscriptions then error('group limit corrupt') end
    -- Bound work to 100 messages per operation even if the caller permits more.
    local candidates = call('ZRANGE', terminal, 0, math.min(limit, 100) - 1)
    local removed, freed = 0, 0
    for _, seq in ipairs(candidates) do
        if number(call('HGET', remaining, seq)) ~= 0 then error('pending message purge') end
        local message = record(seq)
        if message[2] ~= binding then error('topic binding corrupt') end
        for _, group in ipairs(subscribers) do
            local data = group_guard(group)
            if not data then error('group missing') end
            local ready, state, done, dead = group_keys(group)
            if number(seq) > number(data[1]) then
                if not call('ZSCORE', done, seq) then error('nonterminal purge') end
                data[2] = number(data[2]) - 1
                if data[2] < 0 then error('group count corrupt') end
                call('HSET', subscriptions, group, cjson.encode(data))
            end
            call('ZREM', ready, seq)
            call('ZREM', done, seq)
            call('ZREM', dead, seq)
            call('HDEL', state, seq)
        end
        call('XDEL', log, seq .. '-0')
        call('HDEL', records, seq)
        call('HDEL', idem, message[3])
        call('HDEL', remaining, seq)
        call('ZREM', index, seq)
        call('ZREM', terminal, seq)
        freed = freed + number(message[6])
        removed = removed + 1
    end
    local count = number(call('HGET', meta, 'count')) - removed
    local bytes = number(call('HGET', meta, 'bytes')) - freed
    if count < 0 or bytes < 0 then error('retention count corrupt') end
    call('HSET', meta, 'count', count, 'bytes', bytes)
    return {'ok', removed}
end)
