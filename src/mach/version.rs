/*-
 * Copyright: see LICENSE file
 */

use crate::error::Error;
use crate::mach::cputype::{CPU_TYPE_ARM64, CPU_TYPE_X86_64};
use crate::mach::load_command::CommandVariant;
use crate::mach::{Mach, MachO, SingleArch};
if_std! {
    use std::cmp::Ordering;
    use std::collections::VecDeque;
    use std::str::FromStr;
    use std::{env, fmt};
}

#[derive(Eq, Debug)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

if_std! {
    impl fmt::Display for Version {
        // This trait requires `fmt` with this exact signature.
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            // Write strictly the first element into the supplied output
            // stream: `f`. Returns `fmt::Result` which indicates whether the
            // operation succeeded or failed. Note that `write!` uses syntax which
            // is very similar to `println!`.
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }

    impl Ord for Version {
        fn cmp(&self, other: &Self) -> Ordering {
            let mao = self.major.cmp(&other.major);
            let mio = self.minor.cmp(&other.minor);
            let pao = self.patch.cmp(&other.patch);
            if mao == Ordering::Equal && mio == Ordering::Equal {
                pao
            } else if mao == Ordering::Equal {
                mio
            } else {
                mao
            }
        }
    }

    impl PartialOrd for Version {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor && self.patch == other.patch
    }
}
if_std! {
    impl FromStr for Version {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut parts = s
            .trim()
            .split('.')
            .map(|p| p.parse::<u32>().unwrap())
            .take(3)
            .collect::<VecDeque<u32>>();

            Ok(Self {
                major: parts.pop_front().unwrap(),
                minor: parts.pop_front().unwrap_or(0),
                patch: parts.pop_front().unwrap_or(0),
            })
        }
    }
}

impl From<u32> for Version {
    fn from(packed: u32) -> Self {
        // X.Y.Z is encoded in nibbles xxxx.yy.zz
        // 12.6 = 0b0000_0000_0000_1100_0000_0110_0000_0000
        let major = (packed & 0b1111_1111_1111_1111_0000_0000_0000_0000u32) >> 16;
        let minor = (packed & 0b0000_0000_0000_0000_1111_1111_0000_0000u32) >> 8;
        let patch = (packed & 0b0000_0000_0000_0000_0000_0000_1111_1111u32) >> 0;
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl From<MachO<'_>> for Version {
    fn from(b: MachO) -> Self {
        let packed = b
            .load_commands
            .iter()
            .find_map(|c| match c.command {
                CommandVariant::VersionMinMacosx(v) => Some(v.version),
                CommandVariant::BuildVersion(v) => Some(v.minos),
                _ => None,
            })
            .unwrap();
        Self::from(packed)
    }
}

if_std! {
    impl From<Mach<'_>> for Version {
        fn from(b: Mach) -> Self {
            match b {
                Mach::Binary(b) => Version::from(b),
                Mach::Fat(f) => {
                    match f
                    .find(|r| {
                        r.unwrap().cputype
                        == match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
                            Ok("x86_64") => CPU_TYPE_X86_64,
                            Ok("aarch64") => CPU_TYPE_ARM64,
                            _ => panic!("unknown arch"),
                        }
                    })
                    .unwrap()
                    .ok()
                    .unwrap()
                    {
                        SingleArch::MachO(b) => Version::from(b),
                        SingleArch::Archive(_) => panic!("lib is an archive?"),
                    }
                }
            }
        }
    }
}
