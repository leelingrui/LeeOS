static mut IDLE_CNT : u64 = 0;

pub fn idle()
{
    unsafe
    {
        IDLE_CNT += 1;
    }
}