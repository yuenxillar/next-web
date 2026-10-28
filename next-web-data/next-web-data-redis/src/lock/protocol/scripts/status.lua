-- Read the whole lock state in one round trip.
-- KEYS[1] lock key; ARGV[1] owner token.
-- Returns {exists, this owner's hold count, pttl in milliseconds};
-- pttl is -2 when the key does not exist.
if (redis.call('exists', KEYS[1]) == 0) then
  return {0, 0, -2}
end
local count = redis.call('hget', KEYS[1], ARGV[1])
return {1, tonumber(count) or 0, redis.call('pttl', KEYS[1])}
