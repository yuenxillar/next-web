if (redis.call('hexists', KEYS[1], ARGV[2]) == 1) then
  redis.call('pexpire', KEYS[1], ARGV[1])
  return 1
end
return 0
