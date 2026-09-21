if call('EXISTS', meta) == 0 then
    if ARGV[5] ~= '1' then return {'missing'} end
    -- Never overwrite remnants of a damaged namespace.
    for _, key in ipairs({log, records, idem, subscriptions, tokens, remaining}) do
        if call('EXISTS', key) ~= 0 then return {'corrupt'} end
    end
    call('HSET', meta, 'signature', signature, 'dirty', '1', 'count', '0',
        'bytes', '0', 'sequence', '0', 'clock', text(now()))
    for _, key in ipairs({records, idem, subscriptions, tokens, remaining}) do
        call('HSET', key, '_', signature)
    end
    -- An empty stream persists after deleting its initial sentinel entry.
    call('XADD', log, '1-0', 'wire', '')
    call('XDEL', log, '1-0')
    call('HSET', meta, 'sequence', '1', 'dirty', '0')
end
local failure = guard()
if failure then return failure end
return {'ok', text(now())}
