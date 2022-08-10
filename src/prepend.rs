pub fn prepend<T>(to: &mut Vec<T>, pre: T) {
  let mut buf = Vec::with_capacity(to.len() + 1);
  // TODO: Optimize (maybe unsafe)

  buf.push(pre);
  buf.append(to);
  debug_assert!(to.is_empty());

  *to = buf
}
