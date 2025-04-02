/// IPv4 の範囲をサブネットに分割する際に、
/// CIDR ブロックをどのサイズで切るか決めるためのユーティリティ。

/// 範囲[current, end]の中で取れる最大の CIDR ブロックサイズを返す。
pub fn largest_ipv4_block(current: u32, end: u32) -> u8 {
    let tz = current.trailing_zeros();
    let span = (end - current + 1).ilog2_sub1();
    let max_block = tz.min(span);
    (32 - max_block) as u8
}

/// u32用のヘルパートレイト。
/// RIRが出力するIPv4範囲の計算に利用する。
pub trait ILog2Sub1 {
    fn ilog2_sub1(&self) -> u32;
}

impl ILog2Sub1 for u32 {
    fn ilog2_sub1(&self) -> u32 {
        if *self == 0 {
            0
        } else {
            31 - self.leading_zeros()
        }
    }
}
