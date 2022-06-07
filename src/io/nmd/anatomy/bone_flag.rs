use {
    std::{
        fmt::{
            self,
            Display,
            Formatter,
        },
    },
    serde::{
        Deserialize, Serialize
    },
};

use NmdFileBoneFlag::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NmdFileBoneFlag {
    Standard,
    Weapon,
    Finger,
    Face,
    Rot,
    Prot,
    ConstRot,
    Slerp,
    Swing,
    Shit,
    Phit,
    Chit,
    Offset,
    RotOffset,
    ProtOffset,
    ConstRotOffset,
    SlerpOffset,
    Breast,
    Unknown,
}

pub struct NmdFileBoneFlagIterator {
    index: usize,
}

impl NmdFileBoneFlag {
    pub fn is_phys(&self) -> bool {
        match self {
            Swing   => true,
            Breast  => true,
            _       => false
        }
    }

    pub fn iter() -> NmdFileBoneFlagIterator {
        NmdFileBoneFlagIterator {
            index: 0,
        }
    }
}

impl Default for NmdFileBoneFlag {
    fn default() -> Self {
        Self::Standard
    }
}

impl From<u8> for NmdFileBoneFlag {
    fn from(i: u8) -> Self {
        match i {
            0x00            => Standard,
            0x01            => Weapon,
            0x02            => Finger,
            0x03            => Face,
            0x04            => Rot,
            0x05            => Prot,
            0x07            => ConstRot,
            0x09            => Slerp,
            0x0B            => Swing,
            0x0C            => Shit,
            0x0D            => Phit,
            0x12            => Chit,
            0x18            => Offset,
            0x19            => RotOffset,
            0x1A            => ProtOffset,
            0x1B            => ConstRotOffset,
            0x1D            => SlerpOffset,
            0x1E            => Breast,
            unknown         => Unknown,
        }
    }
}

impl From<NmdFileBoneFlag> for u8 {
    fn from(bone_type: NmdFileBoneFlag) -> Self {
        match bone_type {
            Standard        => 0x00,
            Weapon          => 0x01,
            Finger          => 0x02,
            Face            => 0x03,
            Rot             => 0x04,
            Prot            => 0x05,
            ConstRot        => 0x07,
            Slerp           => 0x09,
            Swing           => 0x0B,
            Shit            => 0x0C,
            Phit            => 0x0D,
            Chit            => 0x12,
            Offset          => 0x18,
            RotOffset       => 0x19,
            ProtOffset      => 0x1A,
            ConstRotOffset  => 0x1B,
            SlerpOffset     => 0x1D,
            Breast          => 0x1E,
            Unknown         => 0xFF,
        }
    }
}

impl Display for NmdFileBoneFlag {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}",
            match self {
                Standard        => "Standard",
                Weapon          => "Weapon",
                Finger          => "Finger",
                Face            => "Face",
                Rot             => "Rot",
                Prot            => "Prot",
                ConstRot        => "ConstRot",
                Slerp           => "Slerp",
                Swing           => "Swing",
                Shit            => "Shit",
                Phit            => "Phit",
                Chit            => "Chit",
                Offset          => "Offset",
                RotOffset       => "RotOffset",
                ProtOffset      => "ProtOffset",
                ConstRotOffset  => "ConstRotOffset",
                SlerpOffset     => "SlerpOffset",
                Breast          => "Breast",
                Unknown         => "Unknown",
            }
        )
    }
}

impl NmdFileBoneFlagIterator {
    const FLAGS: &'static [NmdFileBoneFlag] = &[
        Standard,
        Weapon,
        Finger,
        Face,
        Rot,
        Prot,
        ConstRot,
        Slerp,
        Swing,
        Shit,
        Phit,
        Chit,
        Offset,
        RotOffset,
        ProtOffset,
        ConstRotOffset,
        SlerpOffset,
        Breast,
        // Omit `Unknown`
    ];
}

impl Iterator for NmdFileBoneFlagIterator {
    type Item = NmdFileBoneFlag;

    fn next(&mut self) -> Option<NmdFileBoneFlag> {
        self.index += 1;

        Self::FLAGS.get(self.index - 1).cloned()
    }
}
