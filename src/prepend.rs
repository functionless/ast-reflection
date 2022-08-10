use std::iter;

/**
 * Prepends an element to the front of a Vec.
 */
pub fn prepend<T>(to: &mut Vec<T>, element: T) {
  to.splice(0..0, iter::once(element)).for_each(|_| {})
}
