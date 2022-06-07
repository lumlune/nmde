use std::cmp::Ordering;

pub fn group_by<T>(collection: Vec<T>, compare: impl Fn(&T, &T) -> Ordering) -> Vec<Vec<T>> {
    collection.into_iter().fold(vec!(), |mut acc, a| {
        match acc.last_mut() {
            Some(group) => match group.first() {
                Some(b) if compare(&a, b) == Ordering::Equal => group.push(a),
                _ => acc.push(vec!(a))
            }
            _ => acc.push(vec!(a))
        }
        
        acc
    })
}

pub fn sort_then_group_by<T>(mut collection: Vec<T>, compare: impl Fn(&T, &T) -> Ordering) -> Vec<Vec<T>> {
    collection.sort_by(&compare); 

    group_by(collection, compare)
}
