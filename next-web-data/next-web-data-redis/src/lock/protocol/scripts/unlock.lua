-- Release one reentrant level.
-- KEYS[1] lock key; KEYS[2] channel.
-- ARGV[1] owner token; ARGV[2] lease in milliseconds;
-- ARGV[3] publish command (PUBLISH or SPUBLISH); ARGV[4] unlock message.
-- Returns -1 for a non-owner, 0 when another level still holds it, 1 when fully
-- released (the key is deleted and the channel is notified).
if (redis.call('hexists', KEYS[1], ARGV[1]) == 0) then
  return -1
end
local counter = redis.call('hincrby', KEYS[1], ARGV[1], -1)
if (counter > 0) then
  redis.call('pexpire', KEYS[1], ARGV[2])
  return 0
end
redis.call('del', KEYS[1])
local published = redis.pcall(ARGV[3], KEYS[2], ARGV[4])
if (type(published) == 'table' and published.err) then
  redis.call('publish', KEYS[2], ARGV[4])
end
return 1
