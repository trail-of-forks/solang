// SPDX-License-Identifier: Apache-2.0

#![allow(unused_variables)]
#![warn(clippy::renamed_function_params)]

use crate::codegen::cfg::HashTy;
use crate::codegen::{Builtin, Expression};
use crate::emit::binary::Binary;
use crate::emit::stylus::StylusTarget;
use crate::emit::{ContractArgs, TargetRuntime, Variable};
use crate::emit_context;
use crate::sema::ast::{self, CallTy};
use crate::sema::ast::{Function, Namespace, Type};
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{
    ArrayValue, BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue,
    PointerValue,
};
use solang_parser::pt::{Loc, StorageType};
use std::collections::HashMap;

impl<'a> TargetRuntime<'a> for StylusTarget {
    fn get_storage_int(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ty: IntType<'a>,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    fn storage_load(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
        storage_type: &Option<StorageType>,
    ) -> BasicValueEnum<'a> {
        emit_context!(binary);

        let slot_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "slot")
            .unwrap();

        let value_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "value")
            .unwrap();

        binary.builder.build_store(slot_ptr, *slot).unwrap();

        call!("storage_load_bytes32", &[slot_ptr.into(), value_ptr.into()]);

        match ty {
            Type::InternalFunction { .. } => unimplemented!(),
            _ => binary
                .builder
                .build_load(
                    binary.context.custom_width_int_type(256),
                    value_ptr,
                    "value",
                )
                .unwrap(),
        }
    }

    /// Recursively store a type to storage
    fn storage_store(
        &self,
        binary: &Binary<'a>,
        ty: &ast::Type,
        existing: bool,
        slot: &mut IntValue<'a>,
        value: BasicValueEnum<'a>,
        function: FunctionValue<'a>,
        ns: &ast::Namespace,
        storage_type: &Option<StorageType>,
    ) {
        emit_context!(binary);

        let slot_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "slot")
            .unwrap();

        let value_ptr = binary
            .builder
            .build_alloca(slot.get_type(), "value")
            .unwrap();

        binary.builder.build_store(slot_ptr, *slot).unwrap();

        binary.builder.build_store(value_ptr, value).unwrap();

        call!(
            "storage_cache_bytes32",
            &[slot_ptr.into(), value_ptr.into()]
        );

        call!("storage_flush_cache", &[i32_const!(1).into()]);
    }

    /// Recursively clear storage. The default implementation is for slot-based storage
    fn storage_delete(
        &self,
        bin: &Binary<'a>,
        ty: &Type,
        slot: &mut IntValue<'a>,
        function: FunctionValue<'a>,
        ns: &Namespace,
    ) {
        unimplemented!()
    }

    // Bytes and string have special storage layout
    fn set_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        slot: PointerValue<'a>,
        dest: BasicValueEnum<'a>,
    ) {
        unimplemented!()
    }

    fn get_storage_string(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
    ) -> PointerValue<'a> {
        unimplemented!()
    }

    fn set_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue,
        dest: PointerValue,
        dest_ty: BasicTypeEnum,
    ) {
        unimplemented!()
    }

    fn get_storage_extfunc(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: PointerValue<'a>,
        ns: &Namespace,
    ) -> PointerValue<'a> {
        unimplemented!()
    }

    fn get_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        loc: Loc,
        ns: &Namespace,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    fn set_storage_bytes_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue,
        slot: IntValue<'a>,
        index: IntValue<'a>,
        value: IntValue<'a>,
        ns: &Namespace,
        loc: Loc,
    ) {
        unimplemented!()
    }

    fn storage_subscript(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        index: BasicValueEnum<'a>,
        ns: &Namespace,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    fn storage_push(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        val: Option<BasicValueEnum<'a>>,
        ns: &Namespace,
    ) -> BasicValueEnum<'a> {
        unimplemented!()
    }

    fn storage_pop(
        &self,
        bin: &Binary<'a>,
        function: FunctionValue<'a>,
        ty: &Type,
        slot: IntValue<'a>,
        load: bool,
        ns: &Namespace,
        loc: Loc,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!()
    }

    fn storage_array_length(
        &self,
        _bin: &Binary<'a>,
        _function: FunctionValue,
        _slot: IntValue<'a>,
        _elem_ty: &Type,
        _ns: &Namespace,
    ) -> IntValue<'a> {
        unimplemented!()
    }

    /// keccak256 hash
    fn keccak256_hash(
        &self,
        bin: &Binary<'a>,
        src: PointerValue,
        length: IntValue,
        dest: PointerValue,
        ns: &Namespace,
    ) {
        emit_context!(bin);

        call!(
            "native_keccak256",
            &[src.into(), length.into(), dest.into()]
        );
    }

    /// Prints a string
    fn print(&self, bin: &Binary, string: PointerValue, length: IntValue) {
        emit_context!(bin);

        call!("log_txt", &[string.into(), length.into()]);
    }

    /// Return success without any result
    fn return_empty_abi(&self, bin: &Binary) {
        unimplemented!()
    }

    /// Return failure code
    fn return_code<'b>(&self, bin: &'b Binary, ret: IntValue<'b>) {
        emit_context!(bin);

        self.assert_failure(bin, byte_ptr!().const_zero(), i32_zero!());
    }

    /// Return failure without any result
    fn assert_failure(&self, bin: &Binary, data: PointerValue, length: IntValue) {
        emit_context!(bin);

        bin.builder
            .build_store(bin.return_code.unwrap().as_pointer_value(), i32_const!(1))
            .unwrap();

        // smoelius: We must return something here, or else the wasm won't parse. But I'm not sure
        // that returning 0 or 1 makes a difference.
        let one: &dyn BasicValue = &i32_const!(1);
        bin.builder.build_return(Some(one)).unwrap();
    }

    fn builtin_function(
        &self,
        binary: &Binary<'a>,
        function: FunctionValue<'a>,
        builtin_func: &Function,
        args: &[BasicMetadataValueEnum<'a>],
        first_arg_type: Option<BasicTypeEnum>,
        ns: &Namespace,
    ) -> Option<BasicValueEnum<'a>> {
        unimplemented!()
    }

    /// Calls constructor
    fn create_contract<'b>(
        &mut self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        contract_no: usize,
        address: PointerValue<'b>,
        encoded_args: BasicValueEnum<'b>,
        encoded_args_len: BasicValueEnum<'b>,
        contract_args: ContractArgs<'b>,
        ns: &Namespace,
        loc: Loc,
    ) {
        unimplemented!()
    }

    /// call external function
    fn external_call<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        success: Option<&mut BasicValueEnum<'b>>,
        payload: PointerValue<'b>,
        payload_len: IntValue<'b>,
        address: Option<BasicValueEnum<'b>>,
        contract_args: ContractArgs<'b>,
        ty: CallTy,
        ns: &Namespace,
        loc: Loc,
    ) {
        emit_context!(bin);

        let return_data_len = bin
            .builder
            .build_alloca(bin.llvm_type(&ast::Type::Uint(32), ns), "return_data_len")
            .unwrap();

        let name = match ty {
            CallTy::Regular => "call_contract",
            CallTy::Delegate => "delegate_call_contract",
            CallTy::Static => "static_call_contract",
        };

        let mut args: Vec<BasicMetadataValueEnum> =
            vec![address.unwrap().into(), payload.into(), payload_len.into()];

        if matches!(ty, CallTy::Regular) {
            let value = bin
                .builder
                .build_alloca(bin.context.custom_width_int_type(256), "value")
                .unwrap();
            bin.builder
                .build_store(value, contract_args.value.unwrap())
                .unwrap();
            args.push(value.into());
        }

        let gas = gas_calculation(bin, contract_args.gas.unwrap());

        args.extend_from_slice(&[gas.into(), return_data_len.into()]);

        // smoelius: From: https://github.com/OffchainLabs/stylus-sdk-rs/blob/a9d54f5fac69c5dda3ee2fae0562aaefee5c2aad/src/hostio.rs#L77-L78
        // > The return status indicates whether the call succeeded, and is nonzero on failure.
        let status = call!(name, &args, "external call");

        let temp = bin
            .builder
            .build_load(bin.context.i32_type(), return_data_len, "return_data_len")
            .unwrap();
        bin.builder
            .build_store(bin.return_data_len.unwrap().as_pointer_value(), temp)
            .unwrap();

        // smoelius: `status` is a `u8`, but we need an `i32`. Also, as per the comment above, we
        // need to map 0 to 1, and non-zero to 0.
        let status_inverted = status_inverted(
            bin,
            status.try_as_basic_value().left().unwrap().into_int_value(),
        );

        *success.unwrap() = status_inverted.into();
    }

    /// send value to address
    fn value_transfer<'b>(
        &self,
        _bin: &Binary<'b>,
        _function: FunctionValue,
        _success: Option<&mut BasicValueEnum<'b>>,
        _address: PointerValue<'b>,
        _value: IntValue<'b>,
        _ns: &Namespace,
        loc: Loc,
    ) {
        unimplemented!()
    }

    /// builtin expressions
    fn builtin<'b>(
        &self,
        bin: &Binary<'b>,
        expr: &Expression,
        vartab: &HashMap<usize, Variable<'b>>,
        function: FunctionValue<'b>,
        ns: &Namespace,
    ) -> BasicValueEnum<'b> {
        emit_context!(bin);

        match expr {
            Expression::Builtin {
                kind: Builtin::GetAddress,
                ..
            } => {
                let address = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(ns.address_length as u64),
                        "address",
                    )
                    .unwrap();

                call!("contract_address", &[address.into()], "contract_address");

                address.into()
            }
            Expression::Builtin {
                kind: Builtin::Origin,
                ..
            } => {
                let address = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(ns.address_length as u64),
                        "address",
                    )
                    .unwrap();

                call!("tx_origin", &[address.into()], "tx_origin");

                bin.builder
                    .build_load(bin.address_type(ns), address, "tx_origin")
                    .unwrap()
            }
            Expression::Builtin {
                kind: Builtin::Sender,
                ..
            } => {
                let address = bin
                    .builder
                    .build_array_alloca(
                        bin.context.i8_type(),
                        i32_const!(ns.address_length as u64),
                        "address",
                    )
                    .unwrap();

                call!("msg_sender", &[address.into()], "msg_sender");

                bin.builder
                    .build_load(bin.address_type(ns), address, "caller")
                    .unwrap()
            }
            _ => unimplemented!(),
        }
    }

    /// Return the return data from an external call (either revert error or return values)
    fn return_data<'b>(&self, bin: &Binary<'b>, function: FunctionValue<'b>) -> PointerValue<'b> {
        emit_context!(bin);

        // smoelius: To test `return_data_size`, change `any()` to `all()`.
        let size = if cfg!(any()) {
            call!("return_data_size", &[], "return_data_size")
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value()
        } else {
            bin.builder
                .build_load(
                    bin.context.i32_type(),
                    bin.return_data_len.unwrap().as_pointer_value(),
                    "return_data_len",
                )
                .unwrap()
                .into_int_value()
        };

        let return_data = bin
            .builder
            .build_array_alloca(bin.context.i8_type(), size, "return_data")
            .unwrap();

        call!(
            "read_return_data",
            &[return_data.into(), i32_zero!().into(), size.into()],
            "read_return_data"
        );

        call!(
            "vector_new",
            &[size.into(), i32_const!(1).into(), return_data.into(),]
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value()
    }

    /// Return the value we received
    fn value_transferred<'b>(&self, binary: &Binary<'b>, ns: &Namespace) -> IntValue<'b> {
        unimplemented!()
    }

    /// Terminate execution, destroy bin and send remaining funds to addr
    fn selfdestruct<'b>(&self, binary: &Binary<'b>, addr: ArrayValue<'b>, ns: &Namespace) {
        unimplemented!()
    }

    /// Crypto Hash
    fn hash<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        hash: HashTy,
        input: PointerValue<'b>,
        input_len: IntValue<'b>,
        ns: &Namespace,
    ) -> IntValue<'b> {
        emit_context!(bin);

        const FNAME: &str = "native_keccak256";
        const HASHLEN: u64 = 32;

        if hash != HashTy::Keccak256 {
            unimplemented!("{hash:?}");
        }

        let res = bin
            .builder
            .build_array_alloca(bin.context.i8_type(), i32_const!(HASHLEN), "res")
            .unwrap();

        call!(FNAME, &[input.into(), input_len.into(), res.into()], "hash");

        // bytes32 needs to reverse bytes
        let temp = bin
            .builder
            .build_alloca(bin.llvm_type(&ast::Type::Bytes(HASHLEN as u8), ns), "hash")
            .unwrap();

        call!(
            "__beNtoleN",
            &[res.into(), temp.into(), i32_const!(HASHLEN).into()]
        );

        bin.builder
            .build_load(
                bin.llvm_type(&ast::Type::Bytes(HASHLEN as u8), ns),
                temp,
                "hash",
            )
            .unwrap()
            .into_int_value()
    }

    /// Emit event
    fn emit_event<'b>(
        &self,
        bin: &Binary<'b>,
        function: FunctionValue<'b>,
        data: BasicValueEnum<'b>,
        topics: &[BasicValueEnum<'b>],
    ) {
        unimplemented!()
    }

    /// Return ABI encoded data
    fn return_abi_data<'b>(
        &self,
        binary: &Binary<'b>,
        data: PointerValue<'b>,
        data_len: BasicValueEnum<'b>,
    ) {
        emit_context!(binary);

        call!("write_result", &[data.into(), data_len.into()]);

        let zero: &dyn BasicValue = &i32_zero!();
        binary.builder.build_return(Some(zero)).unwrap();
    }
}

use local::{gas_calculation, status_inverted};

mod local {
    #![warn(unused_variables)]

    use super::*;
    use inkwell::IntPredicate;

    pub fn gas_calculation<'a>(bin: &Binary<'a>, gas_value: IntValue<'a>) -> IntValue<'a> {
        if_zero(
            bin,
            bin.context.i64_type(),
            gas_value,
            bin.context.i64_type().const_all_ones(),
            gas_value,
        )
    }

    pub fn status_inverted<'a>(bin: &Binary<'a>, status: IntValue<'a>) -> IntValue<'a> {
        if_zero(
            bin,
            bin.context.i8_type(),
            status,
            bin.context.i32_type().const_int(1, false),
            bin.context.i32_type().const_zero(),
        )
    }

    fn if_zero<'a>(
        bin: &Binary<'a>,
        input_ty: IntType<'a>,
        input: IntValue<'a>,
        zero_output: IntValue<'a>,
        non_zero_output: IntValue<'a>,
    ) -> IntValue<'a> {
        let is_zero = bin
            .builder
            .build_int_compare(IntPredicate::EQ, input, input_ty.const_zero(), "is_zero")
            .unwrap();

        bin.builder
            .build_select(is_zero, zero_output, non_zero_output, "selection")
            .unwrap()
            .into_int_value()
    }
}
