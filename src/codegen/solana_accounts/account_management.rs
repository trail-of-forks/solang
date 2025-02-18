// SPDX-License-Identifier: Apache-2.0

use crate::codegen::cfg::{ControlFlowGraph, Instr};
use crate::codegen::dispatch::solana::SOLANA_DISPATCH_CFG_NAME;
use crate::codegen::{Builtin, Expression};
use crate::sema::ast::{ArrayLength, Function, Namespace, StructType, Type};
use crate::sema::solana_accounts::BuiltinAccounts;
use num_bigint::BigInt;
use num_traits::Zero;
use solang_parser::pt::Loc;
use std::collections::{HashSet, VecDeque};

/// This function walks over the CFG and automates the account management, so developers do not need
/// to do so. For instance, when calling 'new construct{address: addr}()', we construct the correct
/// AccountMeta array with all the accounts the constructor needs.
pub(crate) fn manage_contract_accounts(contract_no: usize, ns: &mut Namespace) {
    let contract_functions = ns.contracts[contract_no].functions.clone();
    let mut constructor_no = None;
    for function_no in &contract_functions {
        if ns.functions[*function_no].is_constructor() {
            constructor_no = Some(*function_no);
        }
        let cfg_no = ns.contracts[contract_no]
            .all_functions
            .get(function_no)
            .copied()
            .unwrap();
        traverse_cfg(
            &mut ns.contracts[contract_no].cfg[cfg_no],
            &ns.functions,
            *function_no,
        );
    }

    if let Some(constructor) = constructor_no {
        let dispatch = ns.contracts[contract_no]
            .cfg
            .iter_mut()
            .find(|cfg| cfg.name == SOLANA_DISPATCH_CFG_NAME)
            .expect("dispatch CFG is always generated");
        traverse_cfg(dispatch, &ns.functions, constructor);
    }
}

/// This function walks over the CFG to process its instructions for the account management.
fn traverse_cfg(cfg: &mut ControlFlowGraph, functions: &[Function], ast_no: usize) {
    if cfg.blocks.is_empty() {
        return;
    }

    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut visited: HashSet<usize> = HashSet::new();
    queue.push_back(0);
    visited.insert(0);

    while let Some(cur_block) = queue.pop_front() {
        for instr in cfg.blocks[cur_block].instr.iter_mut() {
            process_instruction(instr, functions, ast_no);
        }

        for edge in cfg.blocks[cur_block].edges() {
            if !visited.contains(&edge) {
                queue.push_back(edge);
                visited.insert(edge);
            }
        }
    }
}

/// This function processes the instruction, creating the AccountMeta array when possible.
/// Presently, we only check the Instr::Constructor, but more will come later.
fn process_instruction(instr: &mut Instr, functions: &[Function], ast_no: usize) {
    if let Instr::Constructor {
        accounts,
        address,
        constructor_no,
        ..
    } = instr
    {
        if accounts.is_some() || constructor_no.is_none() {
            return;
        }

        let mut account_metas: Vec<Expression> = Vec::new();
        let constructor_func = &functions[constructor_no.unwrap()];
        for (name, account) in constructor_func.solana_accounts.borrow().iter() {
            if name == BuiltinAccounts::DataAccount {
                let address_ref = Expression::GetRef {
                    loc: Loc::Codegen,
                    ty: Type::Ref(Box::new(Type::Address(false))),
                    expr: Box::new(address.as_ref().unwrap().clone()),
                };
                let struct_literal =
                    account_meta_literal(address_ref, account.is_signer, account.is_writer);
                account_metas.push(struct_literal);
            } else if name == BuiltinAccounts::SystemAccount {
                let system_address = Expression::NumberLiteral {
                    loc: Loc::Codegen,
                    ty: Type::Address(false),
                    value: BigInt::zero(),
                };
                let system_ref = Expression::GetRef {
                    loc: Loc::Codegen,
                    ty: Type::Ref(Box::new(Type::Address(false))),
                    expr: Box::new(system_address),
                };
                let struct_literal = account_meta_literal(system_ref, false, false);
                account_metas.push(struct_literal);
            } else {
                let account_index = functions[ast_no]
                    .solana_accounts
                    .borrow()
                    .get_index_of(name)
                    .unwrap();
                let ptr_to_address = accounts_vector_key_at_index(account_index);
                account_metas.push(account_meta_literal(
                    ptr_to_address,
                    account.is_signer,
                    account.is_writer,
                ));
            }
        }
        let metas_vector = Expression::ArrayLiteral {
            loc: Loc::Codegen,
            ty: Type::Array(
                Box::new(Type::Struct(StructType::AccountMeta)),
                vec![ArrayLength::Fixed(BigInt::from(account_metas.len()))],
            ),
            dimensions: vec![account_metas.len() as u32],
            values: account_metas,
        };

        *address = None;
        *accounts = Some(metas_vector);
    } else if let Instr::AccountAccess { loc, name, var_no } = instr {
        // This could have been an Expression::AccountAccess if we had a three-address form.
        // The amount of code necessary to traverse all Instructions and all expressions recursively
        // (Expressions form a tree) makes the usage of Expression::AccountAccess too burdensome.

        // Alternatively, we can create a codegen::Expression::AccountAccess when we have the
        // new SSA IR complete.
        let account_index = functions[ast_no]
            .solana_accounts
            .borrow()
            .get_index_of(name)
            .unwrap();
        let expr = index_accounts_vector(account_index);

        *instr = Instr::Set {
            loc: *loc,
            res: *var_no,
            expr,
        };
    }
}

/// This function automates the process of retrieving 'tx.accounts[index].key'.
pub(crate) fn accounts_vector_key_at_index(index: usize) -> Expression {
    let payer_info = index_accounts_vector(index);

    retrieve_key_from_account_info(payer_info)
}

/// This function retrieves the account key from the AccountInfo struct.
/// The argument should be of type 'Type::Ref(Type::Struct(StructType::AccountInfo))'.
pub(crate) fn retrieve_key_from_account_info(account_info: Expression) -> Expression {
    let address = Expression::StructMember {
        loc: Loc::Codegen,
        ty: Type::Ref(Box::new(Type::Ref(Box::new(Type::Address(false))))),
        expr: Box::new(account_info),
        member: 0,
    };

    Expression::Load {
        loc: Loc::Codegen,
        ty: Type::Ref(Box::new(Type::Address(false))),
        expr: Box::new(address),
    }
}

/// This function automates the process of retrieving 'tx.accounts[index]'.
fn index_accounts_vector(index: usize) -> Expression {
    let accounts_vector = Expression::Builtin {
        loc: Loc::Codegen,
        tys: vec![Type::Array(
            Box::new(Type::Struct(StructType::AccountInfo)),
            vec![ArrayLength::Dynamic],
        )],
        kind: Builtin::Accounts,
        args: vec![],
    };

    Expression::Subscript {
        loc: Loc::Codegen,
        ty: Type::Ref(Box::new(Type::Struct(StructType::AccountInfo))),
        array_ty: Type::Array(
            Box::new(Type::Struct(StructType::AccountInfo)),
            vec![ArrayLength::Dynamic],
        ),
        expr: Box::new(accounts_vector),
        index: Box::new(Expression::NumberLiteral {
            loc: Loc::Codegen,
            ty: Type::Uint(32),
            value: BigInt::from(index),
        }),
    }
}

/// This function creates an AccountMeta struct literal.
pub(crate) fn account_meta_literal(
    address: Expression,
    is_signer: bool,
    is_writer: bool,
) -> Expression {
    Expression::StructLiteral {
        loc: Loc::Codegen,
        ty: Type::Struct(StructType::AccountMeta),
        values: vec![
            address,
            Expression::BoolLiteral {
                loc: Loc::Codegen,
                value: is_writer,
            },
            Expression::BoolLiteral {
                loc: Loc::Codegen,
                value: is_signer,
            },
        ],
    }
}
