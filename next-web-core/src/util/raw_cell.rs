use std::cell::UnsafeCell;
use std::ptr;

pub struct RawCell<T> {
    data: UnsafeCell<T>,
}

impl<T> RawCell<T> {
    /// 创建新的 RawCell
    pub fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
        }
    }

    /// 获取不可变引用（但内部值可能被其他线程修改！）
    ///
    /// # Safety
    /// 调用者必须确保：
    /// 1. 没有其他线程正在写入数据
    /// 2. 如果有其他线程在写入，你读取的值可能是未定义的
    pub fn get(&self) -> &T {
        unsafe { &*self.data.get() }
    }

    /// 获取可变引用
    ///
    /// # Safety
    /// 调用者必须确保：
    /// 1. 没有其他线程同时访问此数据（无论是读还是写）
    /// 2. 编译器不会对访问进行重排导致问题
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    /// 直接写入新值（不返回旧值）
    ///
    /// # Safety
    /// 调用者必须确保：
    /// 1. 没有其他线程同时访问此数据
    /// 2. 写入操作是原子的（对于大于指针的类型可能不是原子的！）
    pub fn set(&self, value: T) {
        unsafe {
            *self.data.get() = value;
        }
    }

    /// 交换值，返回旧值
    ///
    /// # Safety
    /// 调用者必须确保：
    /// 1. 没有其他线程同时访问此数据
    /// 2. 整个交换操作是原子的（实际上不是！）
    pub fn swap(&self, value: T) -> T {
        unsafe { ptr::replace(self.data.get(), value) }
    }

    /// 更新值使用一个函数
    ///
    /// # Safety
    /// 调用者必须确保：
    /// 1. 没有其他线程同时访问此数据
    /// 2. 函数执行期间没有其他线程访问
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        unsafe {
            f(&mut *self.data.get());
        }
    }

    /// 尝试读取（假设读取总是安全的？其实不是！）
    ///
    /// # Safety
    /// 即使只是读取，如果同时有写入，也可能导致：
    /// - 读取到未初始化的内存
    /// - 读取到部分更新的值（tearing）
    /// - 编译器优化导致的奇怪行为
    pub fn read_unsafe(&self) -> T
    where
        T: Copy,
    {
        unsafe { *self.data.get() }
    }
}

// 标记为 Send 和 Sync，允许跨线程共享
unsafe impl<T> Send for RawCell<T> {}
unsafe impl<T> Sync for RawCell<T> {}

// 一些有用的 trait 实现
impl<T> RawCell<T> {
    /// 获取裸指针（最底层的访问）
    pub fn as_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for RawCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 注意：这里读取数据是不安全的！
        write!(f, "RawCell(")?;
        unsafe {
            write!(f, "{:?}", *self.data.get())?;
        }
        write!(f, ")")
    }
}

impl<T> From<T> for RawCell<T> {
    fn from(value: T) -> Self {
        RawCell::new(value)
    }
}

impl<T: Clone> Clone for RawCell<T> {
    fn clone(&self) -> Self {
        let value = unsafe { (*self.data.get()).clone() };
        RawCell::new(value)
    }
}
