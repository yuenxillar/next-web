import org.redisson.Redisson;
import org.redisson.api.RLock;
import org.redisson.api.RedissonClient;
import org.redisson.config.Config;

/** Minimal Redisson 4.7.0 core interoperability probe. */
public final class RedissonInteropProbe {
    public static void main(String[] args) throws Exception {
        if (args.length != 3) {
            throw new IllegalArgumentException("expected: <redis-url> <lock-name> <hold-ms>");
        }

        Config config = new Config();
        config.useSingleServer().setAddress(args[0]);
        RedissonClient client = Redisson.create(config);
        RLock lock = client.getLock(args[1]);
        try {
            lock.lock();
            System.out.println("LOCKED");
            System.out.flush();
            Thread.sleep(Long.parseLong(args[2]));
            lock.unlock();
        } finally {
            client.shutdown();
        }
    }
}
