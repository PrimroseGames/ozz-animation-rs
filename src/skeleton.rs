//!
//! Skeleton data structure definition.
//!

use bimap::BiHashMap;
use std::io::Read;

use crate::archive::Archive;
use crate::base::{DeterministicState, OzzError, OzzIndex};
use crate::math::SoaTransform;

/// Rexported `BiHashMap` in bimap crate.
pub type JointHashMap = BiHashMap<String, i16, DeterministicState, DeterministicState>;

struct JointHashMapWrapper;

#[cfg(feature = "rkyv")]
const _: () = {
    use rkyv::collections::util::Entry;
    use rkyv::ser::{ScratchSpace, Serializer};
    use rkyv::string::ArchivedString;
    use rkyv::vec::{ArchivedVec, VecResolver};
    use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
    use rkyv::{Deserialize, Fallible};

    impl ArchiveWith<JointHashMap> for JointHashMapWrapper {
        type Archived = ArchivedVec<Entry<ArchivedString, i16>>;
        type Resolver = VecResolver;

        unsafe fn resolve_with(field: &JointHashMap, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
            ArchivedVec::resolve_from_len(field.len(), pos, resolver, out);
        }
    }

    impl<S> SerializeWith<JointHashMap, S> for JointHashMapWrapper
    where
        S: ScratchSpace + Serializer + ?Sized,
    {
        fn serialize_with(field: &JointHashMap, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
            return ArchivedVec::serialize_from_iter(field.iter().map(|(key, value)| Entry { key, value }), serializer);
        }
    }

    impl<D> DeserializeWith<ArchivedVec<Entry<ArchivedString, i16>>, JointHashMap, D> for JointHashMapWrapper
    where
        D: Fallible + ?Sized,
    {
        fn deserialize_with(
            field: &ArchivedVec<Entry<ArchivedString, i16>>,
            deserializer: &mut D,
        ) -> Result<JointHashMap, D::Error> {
            let mut result = JointHashMap::with_capacity_and_hashers(
                field.len() as usize,
                DeterministicState::new(),
                DeterministicState::new(),
            );
            for entry in field.iter() {
                result.insert(
                    entry.key.deserialize(deserializer)?,
                    entry.value.deserialize(deserializer)?,
                );
            }
            return Ok(result);
        }
    }
};

///
/// This runtime skeleton data structure provides a const-only access to joint
/// hierarchy, joint names and rest-pose.
///
/// Joint names, rest-poses and hierarchy information are all stored in separate
/// arrays of data (as opposed to joint structures for the RawSkeleton), in order
/// to closely match with the way runtime algorithms use them. Joint hierarchy is
/// packed as an array of parent jont indices (16 bits), stored in depth-first
/// order. This is enough to traverse the whole joint hierarchy. Use
/// iter_depth_first() to implement a depth-first traversal utility.
///
#[derive(Debug, Default)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Skeleton {
    pub joint_rest_poses: Vec<SoaTransform>,
    pub joint_parents: Vec<i16>,
    #[cfg_attr(feature = "rkyv", with(JointHashMapWrapper))]
    pub joint_names: JointHashMap,
}

/// Skeleton meta in `Archive`.
pub struct SkeletonMeta {
    pub version: u32,
    pub num_joints: i32,
    pub joint_names: JointHashMap,
    pub joint_parents: Vec<i16>,
}

impl Skeleton {
    /// `Skeleton` resource file tag for `Archive`.
    #[inline]
    pub fn tag() -> &'static str {
        return "ozz-skeleton";
    }

    #[inline]
    /// `Skeleton` resource file version for `Archive`.
    pub fn version() -> u32 {
        return 2;
    }

    #[cfg(test)]
    pub(crate) fn from_raw(
        joint_rest_poses: Vec<SoaTransform>,
        joint_parents: Vec<i16>,
        joint_names: JointHashMap,
    ) -> Skeleton {
        return Skeleton {
            joint_rest_poses,
            joint_parents,
            joint_names,
        };
    }

    /// Reads a `SkeletonMeta` from a reader.
    pub fn read_meta(archive: &mut Archive<impl Read>, with_joints: bool) -> Result<SkeletonMeta, OzzError> {
        if archive.tag() != Self::tag() {
            return Err(OzzError::InvalidTag);
        }
        if archive.version() != Self::version() {
            return Err(OzzError::InvalidVersion);
        }

        let num_joints: i32 = archive.read()?;
        if num_joints == 0 || !with_joints {
            return Ok(SkeletonMeta {
                version: Self::version(),
                num_joints,
                joint_names: BiHashMap::with_hashers(DeterministicState::new(), DeterministicState::new()),
                joint_parents: Vec::new(),
            });
        }

        let _char_count: i32 = archive.read()?;
        let mut joint_names = BiHashMap::with_capacity_and_hashers(
            num_joints as usize,
            DeterministicState::new(),
            DeterministicState::new(),
        );
        for idx in 0..num_joints {
            joint_names.insert(archive.read::<String>()?, idx as i16);
        }

        let joint_parents: Vec<i16> = archive.read_vec(num_joints as usize)?;

        return Ok(SkeletonMeta {
            version: Self::version(),
            num_joints,
            joint_names,
            joint_parents,
        });
    }

    /// Reads a `Skeleton` from a reader.
    pub fn from_archive(archive: &mut Archive<impl Read>) -> Result<Skeleton, OzzError> {
        let meta = Skeleton::read_meta(archive, true)?;

        let soa_num_joints = (meta.num_joints + 3) / 4;
        let mut joint_rest_poses: Vec<SoaTransform> = Vec::with_capacity(soa_num_joints as usize);
        for _ in 0..soa_num_joints {
            joint_rest_poses.push(archive.read()?);
        }

        return Ok(Skeleton {
            joint_rest_poses,
            joint_parents: meta.joint_parents,
            joint_names: meta.joint_names,
        });
    }

    /// Reads a `Skeleton` from a file.
    #[cfg(not(feature = "wasm"))]
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Skeleton, OzzError> {
        let mut archive = Archive::from_path(path)?;
        return Skeleton::from_archive(&mut archive);
    }

    // Only for wasm test in NodeJS environment.
    #[cfg(all(feature = "wasm", feature = "nodejs"))]
    pub fn from_path(path: &str) -> Result<Skeleton, OzzError> {
        let mut archive = Archive::from_path(path)?;
        return Skeleton::from_archive(&mut archive);
    }
}

impl Skeleton {
    /// Gets the number of joints of `Skeleton`.
    #[inline]
    pub fn num_joints(&self) -> usize {
        return self.joint_parents.len();
    }

    /// Gets the number of joints of `Skeleton` (aligned to 4 * SoA).
    #[inline]
    pub fn num_aligned_joints(&self) -> usize {
        return (self.num_joints() + 3) & !0x3;
    }

    /// Gets the number of soa elements matching the number of joints of `Skeleton`.
    /// This value is useful to allocate SoA runtime data structures.
    #[inline]
    pub fn num_soa_joints(&self) -> usize {
        return (self.joint_parents.len() + 3) / 4;
    }

    /// Gets joint's rest poses. Rest poses are stored in soa format.
    #[inline]
    pub fn joint_rest_poses(&self) -> &[SoaTransform] {
        return &self.joint_rest_poses;
    }

    /// Gets joint's parent indices range.
    #[inline]
    pub fn joint_parents(&self) -> &[i16] {
        return &self.joint_parents;
    }

    /// Gets joint's parent by index.
    #[inline]
    pub fn joint_parent(&self, idx: impl OzzIndex) -> i16 {
        return self.joint_parents[idx.usize()];
    }

    /// Gets joint's name map.
    #[inline]
    pub fn joint_names(&self) -> &JointHashMap {
        return &self.joint_names;
    }

    /// Gets joint's index by name.
    #[inline]
    pub fn joint_by_name(&self, name: &str) -> Option<i16> {
        return self.joint_names.get_by_left(name).map(|idx| *idx);
    }

    /// Gets joint's name by index.
    #[inline]
    pub fn name_by_joint(&self, index: i16) -> Option<&str> {
        return self.joint_names.get_by_right(&index).map(|s| s.as_str());
    }

    /// Test if a joint is a leaf.
    ///
    /// * `joint` - `joint` must be in range [0, num joints].
    ///   Joint is a leaf if it's the last joint, or next joint's parent isn't `joint`.
    #[inline]
    pub fn is_leaf(&self, joint: impl OzzIndex) -> bool {
        let next = joint.usize() + 1;
        return next == self.num_joints() || (self.joint_parents()[next] as i32 != joint.i32());
    }

    /// Iterates through the joint hierarchy in depth-first order.
    ///
    /// * `from` - The joint index to start from. If negative, the iteration starts from the root.
    /// * `f` - The function to call for each joint. The function takes arguments `(joint: i16, parent: i16)`.
    pub fn iter_depth_first<F>(&self, from: impl OzzIndex, mut f: F)
    where
        F: FnMut(i16, i16),
    {
        let mut i = if from.i32() < 0 { 0 } else { from.usize() };
        let mut process = i < self.num_joints();
        while process {
            f(i as i16, self.joint_parent(i));
            i += 1;
            process = i < self.num_joints() && (self.joint_parent(i) as i32 >= from.i32());
        }
    }

    /// Iterates through the joint hierarchy in reverse depth-first order.
    ///
    /// * `f` - The function to call for each joint. The function takes arguments `(joint: i16, parent: i16)`.
    pub fn iter_depth_first_reverse<F>(&self, mut f: F)
    where
        F: FnMut(i16, i16),
    {
        for i in (0..self.num_joints()).rev() {
            let parent = self.joint_parent(i);
            f(i as i16, parent);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::simd::prelude::*;
    use wasm_bindgen_test::*;

    use super::*;
    use crate::math::{SoaQuat, SoaVec3};

    #[test]
    #[wasm_bindgen_test]
    fn test_read_skeleton() {
        let skeleton = Skeleton::from_path("./resource/playback/skeleton.ozz").unwrap();

        assert_eq!(skeleton.joint_rest_poses().len(), 17);
        assert_eq!(
            skeleton.joint_rest_poses()[0].translation,
            SoaVec3 {
                x: f32x4::from_array([-4.01047945e-10, 0.00000000, 0.0710870326, 0.110522307]),
                y: f32x4::from_array([1.04666960, 0.00000000, -8.79573781e-05, -7.82728166e-05]),
                z: f32x4::from_array([-0.0151103791, 0.00000000, 9.85883801e-08, -2.17094467e-10]),
            },
        );
        assert_eq!(
            skeleton.joint_rest_poses()[16].translation,
            SoaVec3 {
                x: f32x4::from_array([0.458143145, 0.117970668, 0.0849116519, 0.00000000]),
                y: f32x4::from_array([2.64545919e-09, 0.148304969, 0.00000000, 0.00000000]),
                z: f32x4::from_array([-4.97557555e-14, -7.47846236e-15, -1.77635680e-17, 0.00000000]),
            }
        );

        assert_eq!(
            skeleton.joint_rest_poses()[0].rotation,
            SoaQuat {
                x: f32x4::from_array([-0.500000000, -0.499999702, -1.41468570e-06, -3.05311332e-14]),
                y: f32x4::from_array([-0.500000000, -0.500000358, -6.93941161e-07, 1.70812796e-22]),
                z: f32x4::from_array([-0.500000000, -0.499999702, 0.000398159056, 1.08420217e-19]),
                w: f32x4::from_array([0.500000000, 0.500000358, 1.00000000, 1.00000000]),
            },
        );
        assert_eq!(
            skeleton.joint_rest_poses()[16].rotation,
            SoaQuat {
                x: f32x4::from_array([-2.20410801e-09, 4.11812209e-07, -6.55128745e-32, 0.00000000]),
                y: f32x4::from_array([4.60687737e-08, -4.11812152e-07, -1.30968591e-21, 0.00000000]),
                z: f32x4::from_array([0.0498105064, 0.707106829, -2.46519033e-32, 0.00000000]),
                w: f32x4::from_array([0.998758733, 0.707106769, 1.00000000, 1.00000000]),
            }
        );

        assert_eq!(
            skeleton.joint_rest_poses()[0].scale,
            SoaVec3 {
                x: f32x4::from_array([1.0, 1.0, 1.0, 1.0]),
                y: f32x4::from_array([1.0, 1.0, 1.0, 1.0]),
                z: f32x4::from_array([1.0, 1.0, 1.0, 1.0]),
            },
        );
        assert_eq!(
            skeleton.joint_rest_poses()[16].scale,
            SoaVec3 {
                x: f32x4::from_array([0.999999940, 1.0, 1.0, 1.0]),
                y: f32x4::from_array([0.999999940, 1.0, 1.0, 1.0]),
                z: f32x4::from_array([1.0, 1.0, 1.0, 1.0]),
            }
        );

        assert_eq!(skeleton.joint_parents().len(), 67);
        assert_eq!(skeleton.joint_parents()[0], -1);
        assert_eq!(skeleton.joint_parents()[66], 65);

        assert_eq!(skeleton.joint_names().len(), 67);
        assert_eq!(skeleton.joint_by_name("Hips"), Some(0));
        assert_eq!(skeleton.joint_by_name("Bip01 R Toe0Nub"), Some(66));
    }

    #[cfg(feature = "rkyv")]
    #[test]
    #[wasm_bindgen_test]
    fn test_rkyv_skeleton() {
        use rkyv::ser::Serializer;
        use rkyv::Deserialize;

        let skeleton = Skeleton::from_path("./resource/playback/skeleton.ozz").unwrap();
        let mut serializer = rkyv::ser::serializers::AllocSerializer::<30720>::default();
        serializer.serialize_value(&skeleton).unwrap();
        let buf = serializer.into_serializer().into_inner();
        let archived = unsafe { rkyv::archived_root::<Skeleton>(&buf) };
        let mut deserializer = rkyv::Infallible::default();
        let skeleton2: Skeleton = archived.deserialize(&mut deserializer).unwrap();

        assert_eq!(skeleton.joint_rest_poses(), skeleton2.joint_rest_poses());
        assert_eq!(skeleton.joint_parents(), skeleton2.joint_parents());
        assert_eq!(skeleton.joint_names(), skeleton2.joint_names());
    }
}
