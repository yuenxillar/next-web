local val = redis.call('get', KEYS[3])
if val ~= false then return tonumber(val) end
if (redis.call('hexists', KEYS[1], ARGV[3]) == 0) then return nil end
local counter = redis.call('hincrby', KEYS[1], ARGV[3], -1)
if (counter > 0) then
  redis.call('pexpire', KEYS[1], ARGV[2]); redis.call('set', KEYS[3], 0, 'px', ARGV[5]); return 0
end
redis.call('del', KEYS[1]); local published = redis.pcall(ARGV[4], KEYS[2], ARGV[1])
if (type(published) == 'table' and published.err) then redis.call('publish', KEYS[2], ARGV[1]) end
redis.call('set', KEYS[3], 1, 'px', ARGV[5]); return 1
