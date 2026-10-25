-- Acquire one reentrant level.
-- KEYS[1] lock key; ARGV[1] lease in milliseconds; ARGV[2] owner token.
-- Returns nil when acquired, otherwise the remaining PTTL in milliseconds.
if ((redis.call('exists', KEYS[1]) == 0) or (redis.call('hexists', KEYS[1], ARGV[2]) == 1)) then
  redis.call('hincrby', KEYS[1], ARGV[2], 1)
  redis.call('pexpire', KEYS[1], ARGV[1])
  return nil
end
return redis.call('pttl', KEYS[1])
