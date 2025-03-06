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

use leo_ast::{AccessExpression, Expression};
use leo_span::{Symbol, sym};

use indexmap::IndexSet;

#[derive(Debug)]
pub struct DeadCodeEliminator {
    /// The set of used variables in the current function body.
    pub(crate) used_variables: IndexSet<Symbol>,

    /// The name of the program currently being processed.
    pub(crate) program_name: Symbol,
}

impl DeadCodeEliminator {
    /// Initializes a new `DeadCodeEliminator`.
    pub(crate) fn new() -> Self {
        Self { used_variables: Default::default(), program_name: Symbol::intern("") }
    }

    pub(crate) fn side_effect_free(expr: &Expression) -> bool {
        use Expression::*;

        match expr {
            Access(AccessExpression::Array(array)) => {
                Self::side_effect_free(&array.array) && Self::side_effect_free(&array.index)
            }
            Access(AccessExpression::AssociatedConstant(_)) => true,
            Access(AccessExpression::AssociatedFunction(func)) => {
                func.arguments.iter().all(Self::side_effect_free)
                    && !matches!(func.variant.name, sym::CheatCode | sym::Mapping | sym::Future)
            }
            Access(AccessExpression::Member(mem)) => Self::side_effect_free(&mem.inner),
            Access(AccessExpression::Tuple(tuple)) => Self::side_effect_free(&tuple.tuple),
            Array(array) => array.elements.iter().all(Self::side_effect_free),
            Binary(bin) => Self::side_effect_free(&bin.left) && Self::side_effect_free(&bin.right),
            Call(..) => {
                // Since calls may halt, be conservative and don't consider any call side effect free.
                false
            }
            Cast(cast) => Self::side_effect_free(&cast.expression),
            Struct(struct_) => {
                struct_.members.iter().all(|mem| mem.expression.as_ref().map_or(true, Self::side_effect_free))
            }
            Ternary(tern) => {
                [&*tern.condition, &*tern.if_true, &*tern.if_false].into_iter().all(Self::side_effect_free)
            }
            Tuple(tuple) => tuple.elements.iter().all(Self::side_effect_free),
            Unary(un) => Self::side_effect_free(&un.receiver),
            Err(_) => false,
            Identifier(_) | Literal(_) | Locator(_) | Unit(_) => true,
        }
    }
}
