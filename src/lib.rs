use index_mem_alloc::MemoryMap;
use solana_program::{
    account_info::AccountInfo, program::invoke, system_instruction, sysvar::rent::Rent,
};
use std::{
    cmp::Ordering,
    fmt::{self, Debug},
    mem::size_of,
    ptr,
};

pub const NULL_NODE: u32 = 0xFFFFFFFF;
pub const NULL_ORDER: u32 = 0xFFFF;

#[repr(C, packed)]
pub struct Node<T: Sized> {
    key: T,
    parent: u32,
    left: u32,
    right: u32,
    sref: u32,
    color: u32,
    link: u32,
}

impl<T: Debug + Copy> Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = self.key;
        let parent = self.parent;
        let left = self.left;
        let right = self.right;
        let sref = self.sref;
        let color = self.color;
        let link = self.link;

        f.debug_struct("Node")
            .field("key", &key)
            .field("parent", &parent)
            .field("left", &left)
            .field("right", &right)
            .field("sref", &sref)
            .field("color", &color)
            .field("link", &link)
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct NodePtr<T: Sized>(pub *mut Node<T>, pub *mut u64);

impl<T> PartialEq for NodePtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.is_null() && other.is_null() {
            return true;
        } else if self.is_null() || other.is_null() {
            return false;
        }
        unsafe { (*self.0).sref == (*other.0).sref }
    }
}

impl<T> NodePtr<T> {
    fn null() -> NodePtr<T> {
        NodePtr(ptr::null_mut(), ptr::null_mut())
    }
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    /// # Safety
    /// This function is really safe
    pub unsafe fn get(entry: *mut u64, sref: u32) -> NodePtr<T> {
        let node_ptr =
            entry.offset(sref as isize * (size_of::<Node<T>>() >> 3) as isize) as *mut Node<T>;
        NodePtr(node_ptr, entry)
    }

    fn new<'a, 'info>(
        mut pt: MemoryMap, //MemoryMap,
        entry: *mut u64,
        non_tree_data_size: usize,
        key: T,
        link: u32,
        tree_acc: &'a AccountInfo<'info>,
        signer: &'a AccountInfo<'info>,
        system_program: &'a AccountInfo<'info>,
    ) -> NodePtr<T> {
        let index = match pt.alloc() {
            Ok(idx) => idx,
            Err(_) => return NodePtr::null(),
        };
        let sref = index;
        let acc_size = tree_acc.data_len();
        let min_size = non_tree_data_size + size_of::<Node<T>>() * (sref + 1);
        if min_size > acc_size {
            let rent = &Rent::default();
            let new_minimum_balance = rent.minimum_balance(min_size);
            let lamports_diff = new_minimum_balance.saturating_sub(tree_acc.lamports());
            invoke(
                &system_instruction::transfer(signer.key, tree_acc.key, lamports_diff),
                &[signer.clone(), tree_acc.clone(), system_program.clone()],
            )
            .unwrap();
            tree_acc.realloc(min_size, true).unwrap();
        }
        unsafe {
            let node_ptr =
                entry.offset((sref * (size_of::<Node<T>>() >> 3)) as isize) as *mut Node<T>;
            *node_ptr = Node {
                key,
                parent: NULL_NODE,
                left: NULL_NODE,
                right: NULL_NODE,
                sref: sref as u32,
                color: 1,
                link,
            };
            NodePtr(node_ptr, entry)
        }
    }
    pub fn left(&self) -> NodePtr<T> {
        if self.is_null() {
            return Self::null();
        }
        unsafe {
            if (self.0.read_unaligned()).left == NULL_NODE {
                return Self::null();
            }
            NodePtr(
                self.1.offset(
                    (self.0.read_unaligned()).left as isize
                        * (std::mem::size_of::<Node<T>>() >> 3) as isize,
                ) as *mut Node<T>,
                self.1,
            )
        }
    }
    pub fn right(&self) -> NodePtr<T> {
        if self.is_null() {
            return Self::null();
        }
        unsafe {
            if (self.0.read_unaligned()).right == NULL_NODE {
                return Self::null();
            }
            NodePtr(
                self.1.offset(
                    (self.0.read_unaligned()).right as isize
                        * (std::mem::size_of::<Node<T>>() >> 3) as isize,
                ) as *mut Node<T>,
                self.1,
            )
        }
    }
    fn parent(&self) -> NodePtr<T> {
        unsafe {
            if self.is_null() || (self.0.read_unaligned()).parent == NULL_NODE {
                return Self::null();
            }
            NodePtr(
                self.1.offset(
                    (self.0.read_unaligned()).parent as isize
                        * (std::mem::size_of::<Node<T>>() >> 3) as isize,
                ) as *mut Node<T>,
                self.1,
            )
        }
    }
    pub fn sref(&self) -> u32 {
        if self.is_null() {
            return NULL_NODE;
        }
        unsafe { self.0.read_unaligned().sref }
    }
    pub fn link(&self) -> u32 {
        if self.is_null() {
            return NULL_ORDER;
        }
        unsafe { (self.0).read_unaligned().link }
    }
    pub fn key(&self) -> T
    where
        T: Copy,
    {
        unsafe { self.0.read_unaligned().key }
    }
    fn set_parent(&mut self, parent: NodePtr<T>) {
        if self.is_null() {
            return;
        }
        unsafe {
            if parent.is_null() {
                (*self.0).parent = NULL_NODE
            } else {
                (*self.0).parent = (*parent.0).sref
            }
        }
    }
    fn set_left(&self, left: NodePtr<T>) {
        if self.is_null() {
            return;
        }
        unsafe {
            if left.is_null() {
                (*self.0).left = NULL_NODE
            } else {
                (*self.0).left = (*left.0).sref
            }
        }
    }
    fn set_right(&self, right: NodePtr<T>) {
        if self.is_null() {
            return;
        }
        unsafe {
            if right.is_null() {
                (*self.0).right = NULL_NODE
            } else {
                (*self.0).right = (*right.0).sref
            }
        }
    }
    fn set_color(&mut self, color: u32) {
        if self.is_null() {
            return;
        }
        unsafe { (*self.0).color = color }
    }
    pub fn is_red_color(&self) -> bool {
        if self.is_null() {
            return false;
        }
        unsafe { (self.0).read_unaligned().color == 1 }
    }
    pub fn is_black_color(&self) -> bool {
        if self.is_null() {
            return true;
        }
        unsafe { (self.0).read_unaligned().color == 0 }
    }
    fn set_red_color(&mut self) {
        self.set_color(1);
    }
    fn set_black_color(&mut self) {
        self.set_color(0);
    }
    fn get_color(&self) -> u32 {
        if self.is_null() {
            return 0;
        }
        unsafe { (self.0.read_unaligned()).color }
    }
    pub fn min_node(self) -> NodePtr<T> {
        let mut temp = self;
        while !temp.left().is_null() {
            temp = temp.left();
        }
        temp
    }
    pub fn max_node(self) -> NodePtr<T> {
        let mut temp = self;
        while !temp.right().is_null() {
            temp = temp.right();
        }
        temp
    }
}
pub struct RBTree {
    pub pt: MemoryMap,
    pub root: *mut u32,
    pub entry: *mut u64,
    /// Size of account data preceding the tree structure.
    /// Used when calculating the total account size during memory allocation.
    /// This value represents the number of bytes reserved for metadata, headers,
    /// or other data stored in the account before the tree nodes.
    pub non_tree_data_size: usize,
}

impl RBTree {
    #[inline]
    fn get_root_sref(&self) -> u32 {
        unsafe { *self.root }
    }
    #[inline]
    fn set_root_sref(&self, new_root: u32) {
        unsafe { *self.root = new_root }
    }
    #[inline]
    fn left_rotate<T: Copy>(&self, mut node: NodePtr<T>) {
        let mut temp = node.right();
        node.set_right(temp.left());
        if !temp.left().is_null() {
            temp.left().set_parent(node);
        }
        temp.set_parent(node.parent());
        if node.sref() == self.get_root_sref() {
            self.set_root_sref(temp.sref());
        } else if node == node.parent().left() {
            node.parent().set_left(temp);
        } else {
            node.parent().set_right(temp);
        }
        temp.set_left(node);
        node.set_parent(temp);
    }
    #[inline]
    fn right_rotate<T: Copy>(&self, mut node: NodePtr<T>) {
        let mut temp = node.left();
        node.set_left(temp.right());

        if !temp.right().is_null() {
            temp.right().set_parent(node);
        }

        temp.set_parent(node.parent());
        if node.sref() == self.get_root_sref() {
            self.set_root_sref(temp.sref());
        } else if node == node.parent().right() {
            node.parent().set_right(temp);
        } else {
            node.parent().set_left(temp);
        }
        temp.set_right(node);
        node.set_parent(temp);
    }
    #[inline]
    fn insert_fixup<T: Copy>(&self, mut node: NodePtr<T>) {
        let mut parent;
        let mut gparent;
        while node.parent().is_red_color() {
            parent = node.parent();
            gparent = parent.parent();
            if parent == gparent.left() {
                let mut uncle = gparent.right();
                if !uncle.is_null() && uncle.is_red_color() {
                    uncle.set_black_color();
                    parent.set_black_color();
                    gparent.set_red_color();
                    node = gparent;
                    continue;
                }
                if parent.right() == node {
                    self.left_rotate(parent);
                    std::mem::swap(&mut parent, &mut node);
                }
                parent.set_black_color();
                gparent.set_red_color();
                self.right_rotate(gparent);
            } else {
                let mut uncle = gparent.left();
                if !uncle.is_null() && uncle.is_red_color() {
                    uncle.set_black_color();
                    parent.set_black_color();
                    gparent.set_red_color();
                    node = gparent;
                    continue;
                }
                if parent.left() == node {
                    self.right_rotate(parent);
                    std::mem::swap(&mut parent, &mut node);
                }
                parent.set_black_color();
                gparent.set_red_color();
                self.left_rotate(gparent);
            }
        }
        self.get_root::<T>().set_black_color();
    }

    pub fn insert_direct<'info, 'a, T: Copy + PartialOrd>(
        &self,
        y: NodePtr<T>,
        key: T,
        link: u32,
        tree_acc: &'a AccountInfo<'info>,
        signer: &'a AccountInfo<'info>,
        system_program: &'a AccountInfo<'info>,
    ) -> u32 {
        let mut node = NodePtr::new(
            self.pt.clone(),
            self.entry,
            self.non_tree_data_size,
            key,
            link,
            tree_acc,
            signer,
            system_program,
        );
        if node.is_null() {
            return NULL_NODE;
        }
        let node_sref = node.sref();
        node.set_parent(y);
        if key < y.key() {
            y.set_left(node);
        } else {
            y.set_right(node);
        }
        node.set_red_color();
        self.insert_fixup(node);
        node_sref
    }
    pub fn insert<'b, 'info, 'a, T: Copy + PartialOrd>(
        &self,
        key: T,
        link: u32,
        tree_acc: &'a AccountInfo<'info>,
        signer: &'a AccountInfo<'info>,
        system_program: &'a AccountInfo<'info>,
    ) -> u32 {
        let mut node = NodePtr::new(
            self.pt.clone(),
            self.entry,
            self.non_tree_data_size,
            key,
            link,
            tree_acc,
            signer,
            system_program,
        );
        if node.is_null() {
            return NULL_NODE;
        }
        let node_sref = node.sref();
        let mut y = NodePtr::null();
        let mut x = self.get_root();
        while !x.is_null() {
            y = x;
            if key < x.key() {
                x = x.left();
            } else {
                x = x.right();
            }
        }
        node.set_parent(y);
        if y.is_null() {
            self.set_root_sref(node.sref());
        } else if key < y.key() {
            y.set_left(node);
        } else {
            y.set_right(node);
        }
        node.set_red_color();
        self.insert_fixup(node);
        node_sref
    }
    #[inline]
    pub fn get_root<T>(&self) -> NodePtr<T> {
        unsafe {
            if *self.root == NULL_NODE {
                return NodePtr::null();
            }
            let node_ptr = self
                .entry
                .offset(*self.root as isize * (std::mem::size_of::<Node<T>>() >> 3) as isize)
                as *mut Node<T>;
            NodePtr(node_ptr, self.entry)
        }
    }
    pub fn find_node<T: Copy + Ord + std::fmt::Display>(&self, key: T) -> NodePtr<T> {
        if self.get_root_sref() == NULL_NODE {
            return NodePtr::null();
        }
        let mut temp = self.get_root();
        loop {
            let next = match key.cmp(&temp.key()) {
                Ordering::Less => temp.left(),
                Ordering::Greater => temp.right(),
                Ordering::Equal => {
                    return temp;
                }
            };
            if next.is_null() {
                break;
            }
            temp = next;
        }
        NodePtr::null()
    }
    pub fn find_new_parent_or_equal<T: Ord + Copy>(&self, key: T) -> (NodePtr<T>, u32) {
        if self.get_root_sref() == NULL_NODE {
            return (NodePtr::null(), 0);
        }
        let mut temp = self.get_root();
        loop {
            let next;
            match key.cmp(&temp.key()) {
                Ordering::Less => {
                    next = temp.left();
                    if next.is_null() {
                        return (temp, 1);
                    }
                }
                Ordering::Greater => {
                    next = temp.right();
                    if next.is_null() {
                        return (temp, 2);
                    }
                }
                Ordering::Equal => {
                    return (temp, 3);
                }
            }
            temp = next;
        }
    }
    #[inline]
    fn delete_fixup<T: Copy>(&self, mut node: NodePtr<T>, mut parent: NodePtr<T>) {
        let mut other;
        while node.sref() != self.get_root_sref() && node.is_black_color() {
            if parent.left() == node {
                other = parent.right();
                if other.is_red_color() {
                    other.set_black_color();
                    parent.set_red_color();
                    self.left_rotate(parent);
                    other = parent.right();
                }
                if other.left().is_black_color() && other.right().is_black_color() {
                    other.set_red_color();
                    node = parent;
                    parent = node.parent();
                } else {
                    if other.right().is_black_color() {
                        other.left().set_black_color();
                        other.set_red_color();
                        self.right_rotate(other);
                        other = parent.right();
                    }
                    other.set_color(parent.get_color());
                    parent.set_black_color();
                    other.right().set_black_color();
                    self.left_rotate(parent);
                    node = self.get_root();
                    break;
                }
            } else {
                other = parent.left();
                if other.is_red_color() {
                    other.set_black_color();
                    parent.set_red_color();
                    self.right_rotate(parent);
                    other = parent.left();
                }
                if other.left().is_black_color() && other.right().is_black_color() {
                    other.set_red_color();
                    node = parent;
                    parent = node.parent();
                } else {
                    if other.left().is_black_color() {
                        other.right().set_black_color();
                        other.set_red_color();
                        self.left_rotate(other);
                        other = parent.left();
                    }
                    other.set_color(parent.get_color());
                    parent.set_black_color();
                    other.left().set_black_color();
                    self.right_rotate(parent);
                    node = self.get_root();
                    break;
                }
            }
        }
        node.set_black_color();
    }
    #[inline]
    pub fn delete<T: Copy>(&mut self, node: NodePtr<T>) {
        let mut child;
        let mut parent;
        let color;
        if !node.left().is_null() && !node.right().is_null() {
            let mut replace = node.right().min_node();
            if node.sref() == self.get_root_sref() {
                self.set_root_sref(replace.sref());
            } else if node.parent().left() == node {
                node.parent().set_left(replace);
            } else {
                node.parent().set_right(replace);
            }

            child = replace.right();
            parent = replace.parent();
            color = replace.get_color();
            if parent == node {
                parent = replace;
            } else {
                if !child.is_null() {
                    child.set_parent(parent);
                }
                parent.set_left(child);
                replace.set_right(node.right());
                node.right().set_parent(replace);
            }
            replace.set_parent(node.parent());
            replace.set_color(node.get_color());
            replace.set_left(node.left());
            node.left().set_parent(replace);
            if color == 0 {
                self.delete_fixup(child, parent);
            }
            self.pt.dealloc(node.sref() as usize).unwrap();
            return;
        }
        if !node.left().is_null() {
            child = node.left();
        } else {
            child = node.right();
        }
        parent = node.parent();
        color = node.get_color();
        if !child.is_null() {
            child.set_parent(parent);
        }
        if self.get_root_sref() == node.sref() {
            self.set_root_sref(child.sref())
        } else if parent.left() == node {
            parent.set_left(child);
        } else {
            parent.set_right(child);
        }

        if color == 0 {
            self.delete_fixup(child, parent);
        }
        self.pt.dealloc(node.sref() as usize).unwrap();
    }

    pub fn remove<T: Copy + Ord + std::fmt::Display>(&mut self, key: T) -> u32 {
        let node = self.find_node(key);
        if node.is_null() {
            return NULL_NODE;
        }
        let link = node.link();
        self.delete(node);

        link
    }
}
