/// 定义一个 `QPS` trait，用于计算每秒查询数（QPS）、操作耗时以及总耗时。
pub trait QPS {
    /// 计算并打印每秒查询数（QPS）。
    ///
    /// # 参数
    /// - `total`: 总操作次数。
    fn qps(&self, total: u64);

    /// 计算并打印总耗时以及每次操作的平均耗时。
    ///
    /// # 参数
    /// - `total`: 总操作次数。
    fn time(&self, total: u64);

    /// 打印从某个时间点到当前的总耗时。
    fn cost(&self);
}

/// 为 `std::time::Instant` 实现 `QPS` trait。
impl QPS for std::time::Instant {
    /// 计算并打印每秒查询数（QPS）。
    ///
    /// # 参数
    /// - `total`: 总操作次数。
    ///
    /// # 实现细节
    /// 使用 `elapsed()` 方法获取从 `Instant` 创建到当前的时间间隔，并根据公式计算 QPS：
    fn qps(&self, total: u64) {
        let time = self.elapsed(); // 获取从 `Instant` 创建到当前的时间间隔 <button class="citation-flag" data-index="5">
        println!(
            "use QPS: {} QPS/s",
            (total as u128 * 1000000000 as u128 / time.as_nanos() as u128)
        );
    }

    /// 计算并打印总耗时以及每次操作的平均耗时。
    ///
    /// # 参数
    /// - `total`: 总操作次数。
    ///
    /// # 实现细节
    /// 使用 `elapsed()` 方法获取总耗时，并计算每次操作的平均耗时（单位：纳秒）。
    fn time(&self, total: u64) {
        let time = self.elapsed(); // 获取从 `Instant` 创建到当前的时间间隔 <button class="citation-flag" data-index="5">
        println!(
            "use Time: {:?} ,each:{} ns/op",
            &time,
            time.as_nanos() / (total as u128)
        );
    }

    /// 打印从某个时间点到当前的总耗时。
    ///
    /// # 实现细节
    /// 使用 `elapsed()` 方法获取从 `Instant` 创建到当前的时间间隔，并直接打印。
    fn cost(&self) {
        let time = self.elapsed(); // 获取从 `Instant` 创建到当前的时间间隔 <button class="citation-flag" data-index="5">
        println!("cost:{:?}", time);
    }
}

/// 单元测试模块，用于验证 `QPS` trait 的功能。
#[cfg(test)]
mod qps_test {

    use std::time::Instant;

    use super::QPS;

    /// 测试 `qps` 方法。
    ///
    /// # 测试内容
    /// 创建一个 `Instant` 对象，执行 1,000,000 次简单的数学运算，然后调用 `qps` 方法计算并打印 QPS。
    #[test]
    fn test_qps() {
        let now = Instant::now(); // 创建一个时间点 <button class="citation-flag" data-index="5">
        for i in 0..1000000 {
            let _ = i * 10; // 执行简单的数学运算以模拟工作负载
        }
        now.qps(1000000); // 调用 `qps` 方法，传入总操作次数
    }
}
