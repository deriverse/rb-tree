use crate::{NULL_NODE, NULL_ORDER};
use index_mem_alloc::MemoryMap;
use solana_program::{account_info::AccountInfo, program::invoke, rent::Rent, system_instruction};
use std::ptr;

#[repr(C)]
pub struct Node<T: Sized> {
    pub key: T,
    pub parent: u32,
    pub left: u32,
    pub right: u32,
    pub sref: u32,
    pub color: u32,
    pub link: u32,
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
    pub fn null() -> NodePtr<T> {
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

    pub fn new<'a, 'info>(
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
            if (*self.0).left == NULL_NODE {
                return Self::null();
            }
            NodePtr(
                self.1.offset(
                    (*self.0).left as isize * (std::mem::size_of::<Node<T>>() >> 3) as isize,
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
            if (*self.0).right == NULL_NODE {
                return Self::null();
            }
            NodePtr(
                self.1.offset(
                    (*self.0).right as isize * (std::mem::size_of::<Node<T>>() >> 3) as isize,
                ) as *mut Node<T>,
                self.1,
            )
        }
    }
    pub fn parent(&self) -> NodePtr<T> {
        unsafe {
            if self.is_null() || (*self.0).parent == NULL_NODE {
                return Self::null();
            }
            NodePtr(
                self.1.offset(
                    (*self.0).parent as isize * (std::mem::size_of::<Node<T>>() >> 3) as isize,
                ) as *mut Node<T>,
                self.1,
            )
        }
    }
    pub fn sref(&self) -> u32 {
        if self.is_null() {
            return NULL_NODE;
        }
        unsafe { (*self.0).sref }
    }
    pub fn link(&self) -> u32 {
        if self.is_null() {
            return NULL_ORDER;
        }
        unsafe { (*self.0).link }
    }
    pub fn key(&self) -> T
    where
        T: Copy,
    {
        unsafe { (*self.0).key }
    }
    pub fn set_parent(&mut self, parent: NodePtr<T>) {
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
    pub fn set_left(&self, left: NodePtr<T>) {
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
    pub fn set_right(&self, right: NodePtr<T>) {
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
    pub fn set_color(&mut self, color: u32) {
        if self.is_null() {
            return;
        }
        unsafe { (*self.0).color = color }
    }
    pub fn is_red_color(&self) -> bool {
        if self.is_null() {
            return false;
        }
        unsafe { (*self.0).color == 1 }
    }
    pub fn is_black_color(&self) -> bool {
        if self.is_null() {
            return true;
        }
        unsafe { (*self.0).color == 0 }
    }
    pub fn set_red_color(&mut self) {
        self.set_color(1);
    }
    pub fn set_black_color(&mut self) {
        self.set_color(0);
    }
    pub fn get_color(&self) -> u32 {
        if self.is_null() {
            return 0;
        }
        unsafe { (*self.0).color }
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
