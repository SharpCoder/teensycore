use crate::*;
use crate::mem::*;
use crate::system::vector::*;
use core::cmp::*;

pub trait Map<K : PartialOrd + PartialEq + Copy, V : Copy> {
    fn insert(&mut self, key: K, value: V);
    fn remove(&mut self, key: K);
    fn get(&self, key: K) -> Option<V>;
    fn get_mut(&mut self, key: K) -> Option<&mut V>;
    fn keys(&self) -> Vector::<K>;
}

pub trait BTree<K : PartialOrd + PartialEq + Copy, V : Copy> {
    fn get_mut(&mut self, target: K) -> Option<&mut V>;
    fn get(&self, target: K) -> Option<V>;
    fn insert(&mut self, target: K, value: V);
    fn keys(&self) -> Vector::<K>;
    fn remove(&mut self, target: K);
}

#[derive(Copy, Clone)]
pub struct MapNode<K : PartialOrd + PartialEq + Copy, V : Copy> {
    item: V,
    key: K,
    left: Option<*mut MapNode<K, V>>,
    right: Option<*mut MapNode<K, V>>,
}


#[derive(Copy, Clone)]
pub struct BTreeMap<K : PartialOrd + PartialEq + Copy, V : Copy> {
    pub root: Option<MapNode<K, V>>,
}

unsafe impl<K : PartialOrd + PartialEq + Copy, V : Copy> Sync for BTreeMap<K, V> where V: Sync  {}

impl <K : PartialOrd + PartialEq + Copy, V : Copy> PartialEq for MapNode<K, V> {
    fn eq(&self, other: &Self) -> bool {
        return self.key == other.key;
    }
}

impl <K : PartialOrd + PartialEq + Copy, V : Copy> PartialOrd for MapNode<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.key > other.key {
            return Some(Ordering::Greater);
        } else if self.key < other.key {
            return Some(Ordering::Less);
        } else {
            return Some(Ordering::Equal);
        }
    }
}

impl <K : PartialOrd + PartialEq + Copy, V : Copy> MapNode<K, V> {
    pub fn new(key: K, val: V) -> *mut Self {
        let ptr = alloc();
        unsafe {
            (*ptr) = MapNode {
                item: val,
                key: key,
                left: None,
                right: None,
            };
        }

        return ptr;
    }

    pub fn size(&self) -> usize {
        let mut result = 1;
        match self.left {
            None => {},
            Some(node) => {
                result += unsafe { node.as_ref().unwrap() }.size();
            }
        };

        match self.right {
            None => {},
            Some(node) => {
                result += unsafe { node.as_ref().unwrap() }.size();
            }
        }

        return result;
    }

    pub fn put_left(&mut self, left: Option<*mut MapNode<K, V>>) {
        self.left = left;
    }

    pub fn put_right(&mut self, right: Option<*mut MapNode<K, V>>) {
        self.right = right;
    }

    pub fn descendant_count(&self) -> u32 {
        return match self.left.is_some() {
            false => 0,
            true => 1,
        } + match self.right.is_some(){
            false => 0,
            true => 1,
        };
    }

    pub fn min_value(&mut self) -> &mut Self {
        if self.left.is_some() {
            let left_node = self.left.unwrap();
            return unsafe { left_node.as_mut().unwrap() }.min_value();
        } else {
            return self;
        }
    }
}

impl <K : PartialOrd + PartialEq + Copy, V : Copy> BTree<K, V> for MapNode<K, V> {
    fn get(&self, target: K) -> Option<V> {
        if self.key == target {
            return Some(self.item);
        } else if self.key > target {
            // Go left
            return match self.left {
                None => None,
                Some(node) => unsafe { node.as_ref().unwrap() }.get(target) 
            };
        } else {
            // Go right
            return match self.right {
                None => None,
                Some(node) => unsafe { node.as_ref().unwrap() }.get(target)
            };
        }
    }

    fn get_mut(&mut self, target: K) -> Option<&mut V> {
        if self.key == target {
            return Some(&mut self.item);
        } else if self.key > target {
            // Go left
            return match self.left {
                None => None,
                Some(node) => unsafe { node.as_mut().unwrap() }.get_mut(target) 
            };
        } else {
            // Go right
            return match self.right {
                None => None,
                Some(node) => unsafe { node.as_mut().unwrap() }.get_mut(target)
            };
        }
    }

    fn insert(&mut self, target: K, value: V) {
        if target == self.key {
            return;
        } else if target < self.key {
            // Insert left
            match self.left {
                None => {
                    self.left = Some(MapNode::new(target, value));
                },
                Some(node) => {
                    unsafe { node.as_mut().unwrap() }.insert(target, value);
                }
            }
        } else {
            // Insert right
            match self.right {
                None => {
                    self.right = Some(MapNode::new(target, value));
                },
                Some(node) => {
                    unsafe { node.as_mut().unwrap() }.insert(target, value);
                }
            }
        }
    }

    fn remove(&mut self, target: K) {
        let self_key = self.key;


        let has_left_child = self.left.is_some();
        let has_right_child = self.right.is_some();
        let left_matches = has_left_child && unsafe { *(self.left.unwrap()) }.key == target;
        let right_matches = has_right_child && unsafe { *(self.right.unwrap()) }.key == target;        

        if self_key == target {
            // Oops, this is the root node, nothing to remove.

        } else if left_matches || right_matches {
            // Check if left has children
            let node = match left_matches {
                true => *(self.left.as_mut().unwrap()),
                false => *(self.right.as_mut().unwrap()), 
            };
            let descendants = unsafe { *node }.descendant_count();

            if descendants == 0 {
                // Snip it
                if left_matches {
                    self.put_left(None);
                } else {
                    self.put_right(None);
                }

                free(node);
            } else if descendants == 1 {
                // Get the single child of the matched node
                // and replace the current node with it
                let replacement;
                if unsafe { *node }.left.is_some() {
                    replacement = unsafe { *node }.left;
                } else {
                    replacement = unsafe { *node }.right;
                }

                if left_matches {
                    self.put_left(replacement);
                } else {
                    self.put_right(replacement);
                }

                free(node);
            } else if descendants == 2 {
                // The hardest case
                // Select the minimum value of the right subtree
                // This is because... I need to learn better borrow checker habits.
                let left_node = unsafe { (node.as_mut().unwrap().left.unwrap()).as_mut().unwrap() };
                let right_node = unsafe { node.as_mut().unwrap().right.unwrap().as_mut().unwrap() };
                let replacement = unsafe { node.as_mut().unwrap().right.unwrap().as_mut().unwrap() }.min_value();


                // Wire up replacement
                if replacement.key != left_node.key {
                    replacement.put_left(Some(left_node));
                }

                if replacement.key != right_node.key {
                    replacement.put_right(Some(right_node));
                }

                if left_matches {
                    self.put_left(Some(replacement));
                } else {
                    self.put_right(Some(replacement));
                }

                free(node);
            }
        } else {
            // Finally, binary search
            if self.key > target && has_left_child {
                let left_node = unsafe { self.left.unwrap().as_mut().unwrap() };
                left_node.remove(target);
            } else if self.key < target && has_right_child {
                let right_node = unsafe { self.right.unwrap().as_mut().unwrap() };
                right_node.remove(target);
            }
        } 
    }

    fn keys(&self) -> Vector::<K> {
        let mut result = vector!(self.key);
        match self.left {
            None => {},
            Some(node) => {
                result.join(&unsafe { node.as_ref().unwrap() }.keys());
            }
        }
        match self.right {
            None => {},
            Some(node) => {
                result.join(&unsafe { node.as_ref().unwrap() }.keys());
            }
        }
        return result;
    }
}

impl <K : PartialOrd + PartialEq + Copy, V : Copy> BTreeMap<K, V> {
    pub fn new() -> Self {
        return BTreeMap {
            root: None,
        };
    }

    pub fn size(&self) -> usize {
        return match &self.root {
            None => 0,
            Some(node) => node.size()
        };
    }
}

impl <K : PartialOrd + PartialEq + Copy, V : Copy> Map<K, V> for BTreeMap<K, V> {
    fn insert(&mut self, key: K, value: V) {
        // If the root node is null, we can insert there
        if self.root.is_none() {
            self.root = Some(MapNode {
                key: key,
                item: value,
                left: None,
                right: None,
            });
        } else {
            self.root.as_mut().unwrap().insert(key, value);
        }
    }

    fn remove(&mut self, key: K) {
        if self.root.is_none() {
            return;
        } else if self.root.unwrap().key == key {
            // NOTE: Nothing to free because root is not on the heap
            self.root = None;
        } else {
            self.root.as_mut().unwrap().remove(key);
        }
    }

    fn get(&self, key: K) -> Option<V> {
        return match &self.root {
            None => None,
            Some(node) => node.get(key),
        };
    }

    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        return match &self.root {
            None => None,
            Some(_) => self.root.as_mut().unwrap().get_mut(key),
        };
    }

    fn keys(&self) -> Vector::<K> {
        return match &self.root {
            None => Vector::new(),
            Some(head) => head.keys(),
        };
    }
}



#[cfg(test)]
mod test { 
    use super::*;
    use crate::system::str::*;

    #[test]
    fn test_map_node() {
        let node = unsafe { MapNode::new(100, 50).as_mut().unwrap() };
        assert_eq!(node.size(), 1);

        node.insert(125, 25);
        assert_eq!(node.size(), 2);

        node.insert(80, 15);
        assert_eq!(node.size(), 3);

        assert_eq!(node.get(80).unwrap(), 15);
        assert_eq!(node.get(125).unwrap(), 25);
        assert_eq!(node.get(100).unwrap(), 50);
        assert_eq!(node.get(374), None);

    }

    #[test]
    fn test_btree_map() {

        let mut map = BTreeMap::<u8, u8>::new();
        map.insert(10, 1);
        map.insert(15, 2);
        map.insert(17, 3);
        
        assert_eq!(map.size(), 3);
        assert_eq!(map.get(10), Some(1));
        assert_eq!(map.get(15), Some(2));
    }

    #[test]
    fn test_btree_keys() {
        let mut map = BTreeMap::new();
        map.insert(10u8, 1u8);
        map.insert(20u8, 2u8);
        map.insert(30u8, 3u8);

        let keys = map.keys();
        assert_eq!(keys.size(), 3);
        assert_eq!(keys.get(0).unwrap(), 10u8);
        assert_eq!(keys.get(1).unwrap(), 20u8);
        assert_eq!(keys.get(2).unwrap(), 30u8);
    }

    #[test]
    fn test_btree_remove() {
        let mut map = BTreeMap::new();
        map.insert(10u8, 1u8);
        map.insert(20u8, 2u8);
        map.insert(8u8, 3u8);
        map.insert(4u8, 4u8);
        map.insert(9u8, 5u8);

        // This will test each primary edge case
        assert_eq!(map.size(), 5);
        map.remove(8);
        assert_eq!(map.size(), 4);
        assert_eq!(map.get(8), None);
        map.remove(9);
        assert_eq!(map.size(), 3);
        assert_eq!(map.get(9), None);
        map.remove(4);
        assert_eq!(map.size(), 2);
        assert_eq!(map.get(4), None);

        // This will test deeply nested deletes
        map.insert(8, 1);
        map.insert(9, 1);
        map.insert(4, 1);
        map.insert(2, 1);
        map.insert(1, 1);
        assert_eq!(map.size(), 7);
        map.remove(2);

        // Test deeply nested left tree
        assert_eq!(map.size(), 6);

        // Test deeply nested right tree
        map.insert(30, 1);
        map.insert(40, 1);
        map.insert(28, 1);
        map.insert(27, 1);
        map.insert(60, 3);
        assert_eq!(map.size(), 11);
        map.remove(60);
        assert_eq!(map.size(), 10);
    }
}