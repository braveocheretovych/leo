// Copyright (C) 2019-2025 Provable Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

use leo_ast::{AccessExpression, Expression, Location, Node, Type};
use leo_span::{Symbol, sym};

use indexmap::IndexSet;

use crate::{SymbolTable, TypeTable};

#[derive(Debug)]
pub struct DeadCodeEliminator<'a> {
    /// The set of used variables in the current function body.
    pub(crate) used_variables: IndexSet<Symbol>,

    /// The name of the program currently being processed.
    pub(crate) program_name: Symbol,

    pub(crate) symbol_table: &'a SymbolTable,

    pub(crate) type_table: &'a TypeTable,
}

impl<'a> DeadCodeEliminator<'a> {
    /// Initializes a new `DeadCodeEliminator`.
    pub(crate) fn new(symbol_table: &'a SymbolTable, type_table: &'a TypeTable) -> Self {
        Self { used_variables: Default::default(), program_name: Symbol::intern(""), symbol_table, type_table }
    }

    fn contains_future_or_record(&self, ty: &Type) -> bool {
        use Type::*;
        match ty {
            Array(array) => self.contains_future_or_record(array.element_type()),
            Composite(composite) => {
                let program = composite.program.unwrap_or(self.program_name);
                let location = Location::new(program, composite.id.name);
                // Struct or record can't contain a record or future, so
                // we don't need to check for that.
                self.symbol_table.lookup_record(location).is_some()
            }

            Tuple(tuple) => tuple.elements().iter().any(|ty| self.contains_future_or_record(ty)),

            Future(..) => true,

            Address | Boolean | Field | Group | Identifier(_) | Integer(_) | Mapping(_) | Scalar | Signature
            | String | Unit | Err => false,
        }
    }

    pub(crate) fn side_effect_free(&self, expr: &Expression) -> bool {
        use Expression::*;

        let sef = |expr| self.side_effect_free(expr);

        match expr {
            Access(AccessExpression::Array(array)) => sef(&array.array) && sef(&array.index),
            Access(AccessExpression::AssociatedConstant(_)) => true,
            Access(AccessExpression::AssociatedFunction(func)) => {
                func.arguments.iter().all(sef)
                    && !matches!(func.variant.name, sym::CheatCode | sym::Mapping | sym::Future)
            }
            Access(AccessExpression::Member(mem)) => sef(&mem.inner),
            Access(AccessExpression::Tuple(tuple)) => sef(&tuple.tuple),
            Array(array) => array.elements.iter().all(sef),
            Binary(bin) => sef(&bin.left) && sef(&bin.right),
            Call(call) => {
                // A function call is side effect free if none of its arguments or returns
                // contain a future or record, and all its arguments are side effect free.
                let ret_ty = self.type_table.get(&call.id()).expect("Type checking should have provided a type.");
                if self.contains_future_or_record(&ret_ty) {
                    false
                } else {
                    call.arguments.iter().all(|arg| {
                        let ty = self.type_table.get(&arg.id()).expect("Type checking should have provided a type.");
                        sef(arg) && !self.contains_future_or_record(&ty)
                    })
                }
            }
            Cast(cast) => sef(&cast.expression),
            Struct(struct_) => struct_.members.iter().all(|mem| mem.expression.as_ref().map_or(true, sef)),
            Ternary(tern) => [&*tern.condition, &*tern.if_true, &*tern.if_false].into_iter().all(sef),
            Tuple(tuple) => tuple.elements.iter().all(sef),
            Unary(un) => sef(&un.receiver),
            Err(_) => false,
            Identifier(_) | Literal(_) | Locator(_) | Unit(_) => true,
        }
    }
}
