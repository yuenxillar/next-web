if (redis.call('del', KEYS[1]) == 1) then
  local published = redis.pcall(ARGV[2], KEYS[2], ARGV[1])
  if (type(published) == 'table' and published.err) then redis.call('publish', KEYS[2], ARGV[1]) end
  return 1
end
return 0
