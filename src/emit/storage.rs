// SPDX-License-Identifier: Apache-2.0

use crate::emit::binary::Binary;
use crate::sema::ast::Type;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{ArrayValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};

/// This trait specifies the methods for managing storage on slot based environments
pub(super) trait StorageSlot {
    fn set_storage(
        &self,
        bin: &Binary,
        slot: PointerValue,
        dest: PointerValue,
        dest_ty: BasicTypeEnum,
    );

    fn get_storage_address<'a>(&self, bin: &Binary<'a>, slot: PointerValue<'a>) -> ArrayValue<'a>;

    /// Clear a particlar storage slot (slot-based storage chains should implement)
    fn storage_delete_single_slot(&self, bin: &Binary, slot: PointerValue);

    /// Recursively load a type from storage for slot based storage
    fn storage_load_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue,
    ) -> BasicValueEnum<'a>;

    /// Recursively store a type to storage for slot-based storage
    fn storage_store_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
    );

    /// Recursively clear bin storage for slot-based storage
    fn storage_delete_slot<'a>(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        slot_ptr: PointerValue<'a>,
        function: FunctionValue<'a>,
    );
}
