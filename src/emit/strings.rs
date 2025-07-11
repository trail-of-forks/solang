// SPDX-License-Identifier: Apache-2.0

use crate::codegen::Expression;
use crate::emit::binary::Binary;
use crate::emit::expression::expression;
use crate::emit::{TargetRuntime, Variable};
use crate::sema::ast::{FormatArg, RetrieveType, StringLocation, Type};
use crate::Target;
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;
use std::collections::HashMap;

/// Implement "...{}...{}".format(a, b)
pub(super) fn format_string<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    args: &[(FormatArg, Expression)],
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
) -> BasicValueEnum<'a> {
    let evaluated_arg = args
        .iter()
        .map(|(spec, arg)| {
            let string_literal = if let Expression::BytesLiteral { value, .. } = arg {
                Some(value)
            } else {
                None
            };
            (
                *spec,
                string_literal,
                arg.ty(),
                expression(target, bin, arg, vartab, function),
            )
        })
        .collect::<Vec<_>>();

    format_evaluated_args(bin, &evaluated_arg, function)
}

pub(super) fn format_evaluated_args<'a>(
    bin: &Binary<'a>,
    evaluated_arg: &[(FormatArg, Option<&Vec<u8>>, Type, BasicValueEnum<'a>)],
    function: FunctionValue<'a>,
) -> BasicValueEnum<'a> {
    // first we need to calculate the space we need
    let mut length = bin.context.i32_type().const_zero();

    for (spec, string_literal, ty, val) in evaluated_arg.iter() {
        let mut ty = ty;
        let val = *val;
        while let Type::UserType(no) = ty {
            ty = &bin.ns.user_types[*no].ty;
        }
        let len = if let Some(string_literal) = *string_literal {
            bin.context
                .i32_type()
                .const_int(string_literal.len() as u64, false)
        } else {
            match ty {
                // bool: "true" or "false"
                Type::Bool => bin.context.i32_type().const_int(5, false),
                // hex encode bytes
                Type::Contract(_) | Type::Address(_) => {
                    let len = if bin.ns.target == Target::Solana && *spec != FormatArg::Hex {
                        base58_size(bin.ns.address_length)
                    } else {
                        2 * bin.ns.address_length
                    };
                    bin.context.i32_type().const_int(len as u64, false)
                }
                Type::Bytes(size) => bin.context.i32_type().const_int(*size as u64 * 2, false),
                Type::String => bin.vector_len(val),
                Type::DynamicBytes => {
                    // will be hex encoded, so double
                    let len = bin.vector_len(val);

                    bin.builder.build_int_add(len, len, "hex_len").unwrap()
                }
                Type::Uint(bits) if *spec == FormatArg::Hex => bin
                    .context
                    .i32_type()
                    .const_int(*bits as u64 / 4 + 2, false),
                Type::Int(bits) if *spec == FormatArg::Hex => bin
                    .context
                    .i32_type()
                    .const_int(*bits as u64 / 4 + 3, false),
                Type::Uint(bits) if *spec == FormatArg::Binary => {
                    bin.context.i32_type().const_int(*bits as u64 + 2, false)
                }
                Type::Int(bits) if *spec == FormatArg::Binary => {
                    bin.context.i32_type().const_int(*bits as u64 + 3, false)
                }
                // bits / 2 is a rough over-estimate of how many decimals we need
                Type::Uint(bits) if *spec == FormatArg::Default => {
                    bin.context.i32_type().const_int(*bits as u64 / 2, false)
                }
                Type::Int(bits) if *spec == FormatArg::Default => bin
                    .context
                    .i32_type()
                    .const_int(*bits as u64 / 2 + 1, false),
                Type::Enum(enum_no) => bin
                    .context
                    .i32_type()
                    .const_int(bin.ns.enums[*enum_no].ty.bits(bin.ns) as u64 / 3, false),
                _ => {
                    let len = unformattable_argument_message(ty).len();
                    bin.context.i32_type().const_int(len as u64, false)
                }
            }
        };

        length = bin.builder.build_int_add(length, len, "").unwrap();
    }

    // allocate the string and
    let vector = bin
        .vector_new(
            length,
            bin.context.i32_type().const_int(1, false),
            None,
            &Type::String,
        )
        .into_pointer_value();

    let output_start = bin.vector_bytes(vector.into());

    // now encode each of the arguments
    let mut output = output_start;

    // format it
    for (spec, string_literal, arg_ty, val) in evaluated_arg.iter() {
        let mut arg_ty = arg_ty;
        let val = *val;
        let is_string_literal = *spec == FormatArg::StringLiteral;
        while let Type::UserType(no) = arg_ty {
            arg_ty = &bin.ns.user_types[*no].ty;
        }
        match (is_string_literal, arg_ty) {
            (false, Type::Bool) => {
                let len = bin
                    .builder
                    .build_select(
                        val.into_int_value(),
                        bin.context.i32_type().const_int(4, false),
                        bin.context.i32_type().const_int(5, false),
                        "bool_length",
                    )
                    .unwrap()
                    .into_int_value();

                let s = bin
                    .builder
                    .build_select(
                        val.into_int_value(),
                        bin.emit_global_string("bool_true", b"true", true),
                        bin.emit_global_string("bool_false", b"false", true),
                        "bool_value",
                    )
                    .unwrap();

                bin.builder
                    .build_call(
                        bin.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    )
                    .unwrap();

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[len], "")
                        .unwrap()
                };
            }
            (false, Type::String) => {
                let s = bin.vector_bytes(val);
                let len = bin.vector_len(val);

                bin.builder
                    .build_call(
                        bin.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    )
                    .unwrap();

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[len], "")
                        .unwrap()
                };
            }
            (false, Type::DynamicBytes) => {
                let s = bin.vector_bytes(val);
                let len = bin.vector_len(val);

                bin.builder
                    .build_call(
                        bin.module.get_function("hex_encode").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    )
                    .unwrap();

                let hex_len = bin.builder.build_int_add(len, len, "hex_len").unwrap();

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[hex_len], "")
                        .unwrap()
                };
            }
            (false, Type::Address(_) | Type::Contract(_)) => {
                // FIXME: For Polkadot we should encode in the SS58 format
                let buf = bin.build_alloca(function, bin.address_type(), "address");
                bin.builder
                    .build_store(buf, val.into_array_value())
                    .unwrap();

                let len = bin
                    .context
                    .i32_type()
                    .const_int(bin.ns.address_length as u64, false);

                let written_len = if bin.ns.target == Target::Solana && *spec != FormatArg::Hex {
                    let calculated_len = base58_size(bin.ns.address_length);
                    let base58_len = bin
                        .context
                        .i32_type()
                        .const_int(calculated_len as u64, false);
                    bin.builder
                        .build_call(
                            bin.module
                                .get_function("base58_encode_solana_address")
                                .unwrap(),
                            &[buf.into(), len.into(), output.into(), base58_len.into()],
                            "",
                        )
                        .unwrap();
                    base58_len
                } else {
                    bin.builder
                        .build_call(
                            bin.module.get_function("hex_encode").unwrap(),
                            &[output.into(), buf.into(), len.into()],
                            "",
                        )
                        .unwrap();

                    bin.context
                        .i32_type()
                        .const_int(2 * bin.ns.address_length as u64, false)
                };

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[written_len], "")
                        .unwrap()
                };
            }
            (false, Type::Bytes(size)) => {
                let buf = bin.build_alloca(function, bin.llvm_type(arg_ty), "bytesN");

                bin.builder.build_store(buf, val.into_int_value()).unwrap();

                let len = bin.context.i32_type().const_int(*size as u64, false);

                bin.builder
                    .build_call(
                        bin.module.get_function("hex_encode_rev").unwrap(),
                        &[output.into(), buf.into(), len.into()],
                        "",
                    )
                    .unwrap();

                let hex_len = bin.builder.build_int_add(len, len, "hex_len").unwrap();

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[hex_len], "")
                        .unwrap()
                };
            }
            (false, Type::Enum(_)) => {
                let val = bin
                    .builder
                    .build_int_z_extend(val.into_int_value(), bin.context.i64_type(), "val_64bits")
                    .unwrap();

                output = bin
                    .builder
                    .build_call(
                        bin.module.get_function("uint2dec").unwrap(),
                        &[output.into(), val.into()],
                        "",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
            }
            (false, Type::Uint(bits)) => {
                if *spec == FormatArg::Default && *bits <= 64 {
                    let val = if *bits == 64 {
                        val.into_int_value()
                    } else {
                        bin.builder
                            .build_int_z_extend(
                                val.into_int_value(),
                                bin.context.i64_type(),
                                "val_64bits",
                            )
                            .unwrap()
                    };

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint2dec").unwrap(),
                            &[output.into(), val.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else if *spec == FormatArg::Default && *bits <= 128 {
                    let val = if *bits == 128 {
                        val.into_int_value()
                    } else {
                        bin.builder
                            .build_int_z_extend(
                                val.into_int_value(),
                                bin.context.custom_width_int_type(128),
                                "val_128bits",
                            )
                            .unwrap()
                    };

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint128dec").unwrap(),
                            &[output.into(), val.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else if *spec == FormatArg::Default {
                    let val = if *bits == 256 {
                        val.into_int_value()
                    } else {
                        bin.builder
                            .build_int_z_extend(
                                val.into_int_value(),
                                bin.context.custom_width_int_type(256),
                                "val_256bits",
                            )
                            .unwrap()
                    };

                    let pval =
                        bin.build_alloca(function, bin.context.custom_width_int_type(256), "int");

                    bin.builder.build_store(pval, val).unwrap();

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint256dec").unwrap(),
                            &[output.into(), pval.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else {
                    let buf = bin.build_alloca(function, bin.llvm_type(arg_ty), "uint");

                    bin.builder.build_store(buf, val.into_int_value()).unwrap();

                    let len = bin.context.i32_type().const_int(*bits as u64 / 8, false);

                    let func_name = if *spec == FormatArg::Hex {
                        "uint2hex"
                    } else {
                        "uint2bin"
                    };

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function(func_name).unwrap(),
                            &[output.into(), buf.into(), len.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }
            }
            (false, Type::Int(bits)) => {
                let val = val.into_int_value();

                let is_negative = bin
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        val,
                        val.get_type().const_zero(),
                        "negative",
                    )
                    .unwrap();

                let entry = bin.builder.get_insert_block().unwrap();
                let positive = bin.context.append_basic_block(function, "int_positive");
                let negative = bin.context.append_basic_block(function, "int_negative");

                bin.builder
                    .build_conditional_branch(is_negative, negative, positive)
                    .unwrap();

                bin.builder.position_at_end(negative);

                // add "-" to output and negate our val
                bin.builder
                    .build_store(output, bin.context.i8_type().const_int('-' as u64, false))
                    .unwrap();

                let minus_len = bin.context.i32_type().const_int(1, false);

                let neg_data = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[minus_len], "")
                        .unwrap()
                };
                let neg_val = bin.builder.build_int_neg(val, "negative_int").unwrap();

                bin.builder.build_unconditional_branch(positive).unwrap();

                bin.builder.position_at_end(positive);

                let data_phi = bin.builder.build_phi(output.get_type(), "data").unwrap();
                let val_phi = bin.builder.build_phi(val.get_type(), "val").unwrap();

                data_phi.add_incoming(&[(&neg_data, negative), (&output, entry)]);
                val_phi.add_incoming(&[(&neg_val, negative), (&val, entry)]);

                if *spec == FormatArg::Default && *bits <= 64 {
                    let val = if *bits == 64 {
                        val_phi.as_basic_value().into_int_value()
                    } else {
                        bin.builder
                            .build_int_z_extend(
                                val_phi.as_basic_value().into_int_value(),
                                bin.context.i64_type(),
                                "val_64bits",
                            )
                            .unwrap()
                    };

                    let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint2dec").unwrap(),
                            &[output_after_minus.into(), val.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else if *spec == FormatArg::Default && *bits <= 128 {
                    let val = if *bits == 128 {
                        val_phi.as_basic_value().into_int_value()
                    } else {
                        bin.builder
                            .build_int_z_extend(
                                val_phi.as_basic_value().into_int_value(),
                                bin.context.custom_width_int_type(128),
                                "val_128bits",
                            )
                            .unwrap()
                    };

                    let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint128dec").unwrap(),
                            &[output_after_minus.into(), val.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else if *spec == FormatArg::Default {
                    let val = if *bits == 256 {
                        val_phi.as_basic_value().into_int_value()
                    } else {
                        bin.builder
                            .build_int_z_extend(
                                val_phi.as_basic_value().into_int_value(),
                                bin.context.custom_width_int_type(256),
                                "val_256bits",
                            )
                            .unwrap()
                    };

                    let pval =
                        bin.build_alloca(function, bin.context.custom_width_int_type(256), "int");

                    bin.builder.build_store(pval, val).unwrap();

                    let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function("uint256dec").unwrap(),
                            &[output_after_minus.into(), pval.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                } else {
                    let buf = bin.build_alloca(function, bin.llvm_type(arg_ty), "int");

                    bin.builder
                        .build_store(buf, val_phi.as_basic_value().into_int_value())
                        .unwrap();

                    let len = bin.context.i32_type().const_int(*bits as u64 / 8, false);

                    let func_name = if *spec == FormatArg::Hex {
                        "uint2hex"
                    } else {
                        "uint2bin"
                    };

                    let output_after_minus = data_phi.as_basic_value().into_pointer_value();

                    output = bin
                        .builder
                        .build_call(
                            bin.module.get_function(func_name).unwrap(),
                            &[output_after_minus.into(), buf.into(), len.into()],
                            "",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }
            }
            (_, _) => {
                let (s, len) =
                    if let (true, Some(string_literal)) = (is_string_literal, string_literal) {
                        (
                            bin.emit_global_string("format_arg", string_literal, true),
                            string_literal.len(),
                        )
                    } else {
                        let message = unformattable_argument_message(arg_ty);
                        (
                            bin.emit_global_string(
                                "unformattable_argument_message",
                                message.as_bytes(),
                                true,
                            ),
                            message.len(),
                        )
                    };
                let len = bin.context.i32_type().const_int(len as u64, false);

                bin.builder
                    .build_call(
                        bin.module.get_function("__memcpy").unwrap(),
                        &[output.into(), s.into(), len.into()],
                        "",
                    )
                    .unwrap();

                output = unsafe {
                    bin.builder
                        .build_gep(bin.context.i8_type(), output, &[len], "")
                        .unwrap()
                };
            }
        }
    }

    // write the final length into the vector
    let length = bin
        .builder
        .build_int_sub(
            bin.builder
                .build_ptr_to_int(output, bin.context.i32_type(), "end")
                .unwrap(),
            bin.builder
                .build_ptr_to_int(output_start, bin.context.i32_type(), "begin")
                .unwrap(),
            "datalength",
        )
        .unwrap();

    let data_len = unsafe {
        bin.builder
            .build_gep(
                bin.module.get_struct_type("struct.vector").unwrap(),
                vector,
                &[
                    bin.context.i32_type().const_zero(),
                    bin.context.i32_type().const_zero(),
                ],
                "data_len",
            )
            .unwrap()
    };

    bin.builder.build_store(data_len, length).unwrap();

    vector.into()
}

fn unformattable_argument_message(ty: &Type) -> String {
    format!("<unformattable argument of type {ty:?}>")
}

/// Load a string from expression or create global
pub(super) fn string_location<'a, T: TargetRuntime<'a> + ?Sized>(
    target: &T,
    bin: &Binary<'a>,
    location: &StringLocation<Expression>,
    vartab: &HashMap<usize, Variable<'a>>,
    function: FunctionValue<'a>,
) -> (PointerValue<'a>, IntValue<'a>) {
    match location {
        StringLocation::CompileTime(literal) => (
            bin.emit_global_string("const_string", literal, true),
            bin.context
                .i32_type()
                .const_int(literal.len() as u64, false),
        ),
        StringLocation::RunTime(e) => {
            if let Expression::BytesLiteral { value, .. } = e.as_ref() {
                (
                    bin.emit_global_string("const_string", value, true),
                    bin.context.i32_type().const_int(value.len() as u64, false),
                )
            } else {
                let v = expression(target, bin, e, vartab, function);

                (bin.vector_bytes(v), bin.vector_len(v))
            }
        }
    }
}

fn base58_size(length: usize) -> usize {
    length * 138 / 100
}
