return mutate(function(timestamp)
    local group, limit, lease = ARGV[5], number(ARGV[6]), number(ARGV[7])
    if not group_guard(group) then return {'subscription'} end
    local ready, state = group_keys(group)
    local candidates = call('ZRANGEBYSCORE', ready, '-inf', timestamp, 'LIMIT', 0, limit)
    local response = {'ok'}
    local bytes = 0
    for _, seq in ipairs(candidates) do
        local stored = call('HGET', state, seq)
        local data = stored and cjson.decode(stored) or {0, '', 0, '', 0}
        local attempts = number(data[1])
        if attempts >= max_attempts then
            finish_delivery(group, seq, data, 'delivery.max_attempts', timestamp)
        else
            local wire = frame(seq)
            if bytes + #wire > 4194304 then break end
            local token = ARGV[7 + #response]
            if not token or call('HEXISTS', tokens, token) ~= 0 then error('lease collision') end
            if data[2] ~= '' then call('HDEL', tokens, data[2]) end
            local expires = timestamp + lease
            local attempt = attempts + 1
            call('HSET', state, seq, cjson.encode({attempt, token, text(expires), '', 0}))
            call('HSET', tokens, token, cjson.encode({group, seq}))
            call('ZADD', ready, expires, seq)
            response[#response + 1] = {wire, attempt, text(expires), token}
            bytes = bytes + #wire
        end
    end
    return response
end)
