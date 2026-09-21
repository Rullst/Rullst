return mutate(function(timestamp)
    local topic, key, fingerprint, id, offset, wire = ARGV[5], ARGV[6], ARGV[7], ARGV[8], number(ARGV[9]), ARGV[10]
    local published = text(timestamp)
    local binding = topic .. ':' .. key
    local existing = call('HGET', idem, binding)
    if existing then
        local message = record(existing)
        if message[4] ~= fingerprint then return {'conflict'} end
        return {'ok', message[1], 1, message[5]}
    end
    local count = number(call('HGET', meta, 'count'))
    local bytes = number(call('HGET', meta, 'bytes'))
    if count >= max_messages then return {'messages'} end
    if #wire > 1060000 or bytes + #wire > 67108864 then return {'bytes'} end
    local sequence = number(call('HGET', meta, 'sequence')) + 1
    if sequence > 9007199254740000 then return {'sequence'} end
    local seq = text(sequence)
    local encoded = ''
    for shift = 7, 0, -1 do
        encoded = encoded .. string.char(math.floor(timestamp / (256 ^ shift)) % 256)
    end
    if offset < 18 or offset + 8 > #wire then error('wire timestamp offset') end
    wire = string.sub(wire, 1, offset) .. encoded .. string.sub(wire, offset + 9)
    local index, groups, terminal = topic_keys(topic)
    local subscribers = call('SMEMBERS', groups)
    if #subscribers > max_subscriptions then error('group limit corrupt') end
    table.sort(subscribers)
    for _, group in ipairs(subscribers) do
        local data = group_guard(group)
        if not data then error('subscription missing') end
        local ready = group_keys(group)
        call('ZADD', ready, timestamp, seq)
        data[2] = number(data[2]) + 1
        call('HSET', subscriptions, group, cjson.encode(data))
    end
    call('XADD', log, seq .. '-0', 'wire', wire)
    call('HSET', records, seq, cjson.encode({id, topic, binding, fingerprint, published, #wire}))
    call('HSET', idem, binding, seq)
    call('HSET', remaining, seq, #subscribers)
    call('ZADD', index, sequence, seq)
    if #subscribers == 0 then call('ZADD', terminal, sequence, seq) end
    call('HSET', meta, 'sequence', seq, 'count', count + 1, 'bytes', bytes + #wire)
    return {'ok', id, 0, published}
end)
