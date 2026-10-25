-- Renew the lease of a lock this owner still holds.
-- KEYS[1] lock key; ARGV[1] lease in milliseconds; ARGV[2] owner token.
-- Returns 1 when renewed, 0 when the owner no longer holds a level.
if (redis.call('hexists', KEYS[1], ARGV[2]) == 1) then
  redis.call('pexpire', KEYS[1], ARGV[1])
  return 1
end
return 0
