

           / /\/ / \ \/\_/ \|
      ___/                   ---
      \   ###   ##  #   #  #   /
      -   #  # #  # # # #  #   -
      \   ###  #  # ## ##       \
      /   #     ##  #   #  #   --
      --                     /
         \/\   /\ / \ | \/ /



# Usage

Edit the included `example-config.json` to suit your needs (pool address,
number of threads, etc), then run:

`powhasher -c your-config.json`

While the hasher is running, press Enter to get statistics.

# What is it?

This is a simple CLI miner for modern x86 CPUs, powered by the
[yellowsun](https://github.com/kazcw/yellowsun) CryptoNight hash implementation
and the [cn-stratum](https://github.com/kazcw/cn-stratum) pool client. It once
hosted unusually fast Cn/Cnv1 core loops I wrote in assembly, but the current
Cnv2 backend is pure Rust based on stdsimd intrinsics.

# Supported platforms

If you'd like to use this on a platform that isn't Linux, all it needs is a
hugepage mmap implementation for your weird choice of operating systems. PRs
for the [yellowsun](https://github.com/kazcw/yellowsun) project are welcome!
