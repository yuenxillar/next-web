-- Delete the lock regardless of owner.
-- KEYS[1] lock key; KEYS[2] channel.
-- ARGV[1] publish command (PUBLISH or SPUBLISH); ARGV[2] unlock message.
-- Returns 1 when a key was deleted and the channel was notified, else 0.
if (redis.call('del', KEYS[1]) == 0) then
  return 0
end
local published = redis.pcall(ARGV[1], KEYS[2], ARGV[2])
if (type(published) == 'table' and published.err) then
  redis.call('publish', KEYS[2], ARGV[2])
end
return 1
