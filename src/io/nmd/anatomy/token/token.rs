use {
    crate::io::nmd::anatomy::token::{
        NmdFileTokenFormat,
        NmdFileTokenValue,
    }
};

/*
 * TODO:
 *
 * ~ (Maybe) Split header/bone token types, names are unwieldy
 */

macro_rules! token_value {
    ($offset:expr, $format:expr $(, $rest:ident)*) => {
        token_value!(@ {
            offset: $offset,
            format: $format,
        } $(, $rest)*)
    };

    (@ { $($fields:tt)* }, $flag:ident $(, $rest:ident)*) => {
        token_value!(@ {
            $($fields)*
            $flag: true,
        } $(, $rest)*)
    };

    (@ { $($fields:tt)* }) => {
        NmdFileTokenValue {
            $($fields)*
            ..NmdFileTokenValue::DEFAULT
        }
    };
}

#[derive(Debug)]
pub enum NmdFileToken {
    HeaderBoneCount,
    HeaderBlobDataAddress,
    HeaderBoneNameDataAddress,
    HeaderBoneDataAddress,
    HeaderBoneCountEcho,
    BoneCollisionData,
    BoneTranslationX,
    BoneTranslationY,
    BoneTranslationZ,
    BoneUnknownDataA, // Maybe constant: [0x00, 0x00, 0x80, 0x3F]
    BoneRotationX,
    BoneRotationY,
    BoneRotationZ,
    BoneUnknownDataB, // Maybe constant: [0x00, 0x00, 0x00, 0x00]
    BoneNameAddress,
    BoneUnknownDataC, // Likely part of the preceding address
    BonePhysicsDataAddress,
    BoneUnknownDataD, // Likely part of the preceding address
    BoneTranslationXNext,
    BoneGravityX,
    BoneGravityY,
    BonePhysicsConstraintXPos,
    BonePhysicsConstraintXNeg,
    BonePhysicsConstraintYPos,
    BonePhysicsConstraintYNeg,
    BoneUnknownDataE, // Not constant, sparse non-zero bytes (x 19)
    BoneFlag,
    BoneParentId,
    BoneId,
    BoneUnknownDataF, // Maybe constant: [0x00, 0x00, ...] (x 12)
}

impl NmdFileToken {
    pub const fn value(&self) -> NmdFileTokenValue {
        use NmdFileToken::*;
        use NmdFileTokenFormat::*;

        match self {
            HeaderBoneCount             => token_value!(0x000A, Short),
            HeaderBlobDataAddress       => token_value!(0x0010, Address),
            HeaderBoneNameDataAddress   => token_value!(0x0014, Address),
            HeaderBoneDataAddress       => token_value!(0x0018, Address),
            HeaderBoneCountEcho         => token_value!(0x001C, Short),
            BoneCollisionData           => token_value!(0x0000, Bytes(16),  is_relative),
            BoneTranslationX            => token_value!(0x0010, Float,      is_relative),
            BoneTranslationY            => token_value!(0x0014, Float,      is_relative),
            BoneTranslationZ            => token_value!(0x0018, Float,      is_relative),
            BoneUnknownDataA            => token_value!(0x001C, Bytes(4),   is_relative),
            BoneRotationX               => token_value!(0x0020, Float,      is_relative),
            BoneRotationY               => token_value!(0x0024, Float,      is_relative),
            BoneRotationZ               => token_value!(0x0028, Float,      is_relative),
            BoneUnknownDataB            => token_value!(0x002C, Bytes(4),   is_relative),
            BoneNameAddress             => token_value!(0x0030, Address,    is_relative),
            BoneUnknownDataC            => token_value!(0x0034, Bytes(4),   is_relative),
            BonePhysicsDataAddress      => token_value!(0x0038, Address,    is_relative),
            BoneUnknownDataD            => token_value!(0x003C, Bytes(4),   is_relative),
            BoneTranslationXNext        => token_value!(0x0040, Float,      is_relative),
            BoneGravityX                => token_value!(0x0044, Short,      is_relative),
            BoneGravityY                => token_value!(0x0046, Short,      is_relative),
            BonePhysicsConstraintXPos   => token_value!(0x0048, Byte,       is_relative),
            BonePhysicsConstraintXNeg   => token_value!(0x0049, Byte,       is_relative),
            BonePhysicsConstraintYPos   => token_value!(0x004A, Byte,       is_relative),
            BonePhysicsConstraintYNeg   => token_value!(0x004B, Byte,       is_relative),
            BoneUnknownDataE            => token_value!(0x004C, Bytes(19),  is_relative),
            BoneFlag                    => token_value!(0x005F, Byte,       is_relative),
            BoneParentId                => token_value!(0x0060, Short,      is_relative),
            BoneId                      => token_value!(0x0062, Short,      is_relative),
            BoneUnknownDataF            => token_value!(0x0064, Bytes(12),  is_relative),
        }
    }
}

