use nalgebra::Vector3;

/// Clamp `val` into the range defined by `lo` and `hi`.
pub fn clamp(val: f32, lo: f32, hi: f32) -> f32 {
    val.max(lo).min(hi)
}

pub fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

pub fn dot2(vec: &Vector3<f32>) -> f32 {
    vec.dot(&vec)
}

/// Partition a slice so that elements that pass the predicate all lie before elements that fail
/// it. Returns the index where the elements that failed the predicate start.
pub fn partition_by<T, Pred>(vec: &mut [T], pred: Pred) -> Option<usize>
where
    Pred: Fn(&T) -> bool,
{
    vec.iter()
        .enumerate()
        .find_map(|(ix, t)| if !pred(t) { Some(ix) } else { None })
        .map(|mut first| {
            for i in first + 1..vec.len() {
                if pred(&vec[i]) {
                    vec.swap(i, first);
                    first += 1;
                }
            }
            first
        })
}

#[test]
fn test_partition_by() {
    {
        let mut vec = vec![1, 2, 3, 4, 5, 6];
        assert_eq!(partition_by(&mut vec, |x| x % 2 == 1), Some(3));
        assert_eq!(vec, vec![1, 3, 5, 4, 2, 6]);
    }
    {
        let mut vec = vec![2, 2, 1, 1];
        assert_eq!(partition_by(&mut vec, |x| x % 2 == 1), Some(2));
        assert_eq!(vec, vec![1, 1, 2, 2]);
    }
    {
        let mut vec = vec![1, 2, 2, 1, 1, 2];
        assert_eq!(partition_by(&mut vec, |x| x % 2 == 1), Some(3));
        assert_eq!(vec, vec![1, 1, 1, 2, 2, 2]);
    }
    {
        let mut vec = vec![1, 2, 1, 1, 1, 2];
        assert_eq!(partition_by(&mut vec, |x| x % 2 == 1), Some(4));
        assert_eq!(vec, vec![1, 1, 1, 1, 2, 2]);
    }
}
