use crate::*;
use crate::space::*;
use crate::metta::*;
use crate::metta::text::Tokenizer;
use crate::metta::types::{get_atom_types, get_meta_type};
use crate::common::multitrie::MultiTrie;
use crate::space::grounding::atom_to_trie_key;
#[cfg(feature = "pkg_mgmt")]
use crate::metta::runner::stdlib::{grounded_op, unit_result, regex};
use crate::metta::runner::{arithmetics::*, Metta};
use crate::common::shared::Shared;

use std::convert::TryInto;

#[derive(Clone, Debug)]
pub struct UniqueAtomOp {}

grounded_op!(UniqueAtomOp, "unique-atom");

impl Grounded for UniqueAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for UniqueAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("unique expects single expression atom as an argument");
        let expr = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?;

        let mut atoms: Vec<Atom> = expr.children().into();
        let mut set = GroundingSpace::new();
        atoms.retain(|x| {
            let not_contained = set.query(x).is_empty();
            if not_contained { set.add(x.clone()) };
            not_contained
        });
        Ok(vec![Atom::expr(atoms)])
    }
}

#[derive(Clone, Debug)]
pub struct UnionAtomOp {}

grounded_op!(UnionAtomOp, "union-atom");

impl Grounded for UnionAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for UnionAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("union expects and executable LHS and RHS atom");
        let mut lhs: Vec<Atom> = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children().into();
        let rhs: Vec<Atom> = TryInto::<&ExpressionAtom>::try_into(args.get(1).ok_or_else(arg_error)?)?.children().into();

        lhs.extend(rhs);

        Ok(vec![Atom::expr(lhs)])
    }
}

#[derive(Clone, Debug)]
pub struct IntersectionAtomOp {}

grounded_op!(IntersectionAtomOp, "intersection-atom");

impl Grounded for IntersectionAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for IntersectionAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("intersection expects and executable LHS and RHS atom");
        let mut lhs: Vec<Atom> = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children().into();
        let rhs = TryInto::<&ExpressionAtom>::try_into(args.get(1).ok_or_else(arg_error)?)?.children();

        let mut rhs_index: MultiTrie<SymbolAtom, Vec<usize>> = MultiTrie::new();
        for (index, rhs_item) in rhs.iter().enumerate() {
            let k = atom_to_trie_key(&rhs_item);
            // FIXME this should
            // a) use a mutable value endpoint which the MultiTrie does not support atm
            // b) use a linked list, which Rust barely supports atm
            let r = rhs_index.get(&k).next();
            match r.cloned() {
                Some(bucket) => {
                    rhs_index.remove(&k, &bucket);
                    let mut nbucket = bucket;
                    nbucket.push(index);
                    let nbucket = nbucket;
                    rhs_index.insert(k, nbucket);
                }
                None => { rhs_index.insert(k, vec![index]) }
            }
        }

        lhs.retain(|candidate| {
            let k = atom_to_trie_key(candidate);
            let r = rhs_index.get(&k).next();
            match r.cloned() {
                None => { false }
                Some(bucket) => {
                    match bucket.iter().position(|item| &rhs[*item] == candidate) {
                        None => { false }
                        Some(i) => {
                            rhs_index.remove(&k, &bucket);
                            if bucket.len() > 1 {
                                let mut nbucket = bucket;
                                nbucket.remove(i);
                                rhs_index.insert(k, nbucket);
                            }
                            true
                        }
                    }
                }
            }
        });

        Ok(vec![Atom::expr(lhs)])
    }
}

#[derive(Clone, Debug)]
pub struct MaxAtomOp {}

grounded_op!(MaxAtomOp, "max-atom");

impl Grounded for MaxAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_NUMBER])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for MaxAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("max-atom expects one argument: expression");
        let children = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children();
        if children.is_empty() {
            Err(ExecError::from("Empty expression"))
        } else {
            children.into_iter().fold(Ok(f64::NEG_INFINITY), |res, x| {
                match (res, Number::from_atom(x)) {
                    (res @ Err(_), _) => res,
                    (_, None) => Err(ExecError::from("Only numbers are allowed in expression")),
                    (Ok(max), Some(x)) => Ok(f64::max(max, x.into())),
                }
            })
        }.map(|max| vec![Atom::gnd(Number::Float(max))])
    }
}

#[derive(Clone, Debug)]
pub struct MinAtomOp {}

grounded_op!(MinAtomOp, "min-atom");

impl Grounded for MinAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_NUMBER])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for MinAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("min-atom expects one argument: expression");
        let children = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children();
        if children.is_empty() {
            Err(ExecError::from("Empty expression"))
        } else {
            children.into_iter().fold(Ok(f64::INFINITY), |res, x| {
                match (res, Number::from_atom(x)) {
                    (res @ Err(_), _) => res,
                    (_, None) => Err(ExecError::from("Only numbers are allowed in expression")),
                    (Ok(min), Some(x)) => Ok(f64::min(min, x.into())),
                }
            })
        }.map(|min| vec![Atom::gnd(Number::Float(min))])
    }
}

#[derive(Clone, Debug)]
pub struct SizeAtomOp {}

grounded_op!(SizeAtomOp, "size-atom");

impl Grounded for SizeAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_NUMBER])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for SizeAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("size-atom expects one argument: expression");
        let children = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children();
        let size = children.len();
        Ok(vec![Atom::gnd(Number::Integer(size as i64))])
    }
}

#[derive(Clone, Debug)]
pub struct IndexAtomOp {}

grounded_op!(IndexAtomOp, "index-atom");

impl Grounded for IndexAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_NUMBER, ATOM_TYPE_ATOM])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for IndexAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("index-atom expects two arguments: expression and atom");
        let children = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children();
        let index = args.get(1).and_then(Number::from_atom).ok_or_else(arg_error)?;
        match children.get(Into::<i64>::into(index) as usize) {
            Some(atom) => Ok(vec![atom.clone()]),
            None => Err(ExecError::from("Index is out of bounds")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SubtractionAtomOp {}

grounded_op!(SubtractionAtomOp, "subtraction-atom");

impl Grounded for SubtractionAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION, ATOM_TYPE_EXPRESSION])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for SubtractionAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("subtraction expects and executable LHS and RHS atom");
        let mut lhs: Vec<Atom> = TryInto::<&ExpressionAtom>::try_into(args.get(0).ok_or_else(arg_error)?)?.children().into();
        let rhs = TryInto::<&ExpressionAtom>::try_into(args.get(1).ok_or_else(arg_error)?)?.children();

        let mut rhs_index: MultiTrie<SymbolAtom, Vec<usize>> = MultiTrie::new();
        for (index, rhs_item) in rhs.iter().enumerate() {
            let k = atom_to_trie_key(&rhs_item);
            // FIXME this should
            // a) use a mutable value endpoint which the MultiTrie does not support atm
            // b) use a linked list, which Rust barely supports atm
            let r = rhs_index.get(&k).next();
            match r.cloned() {
                Some(bucket) => {
                    rhs_index.remove(&k, &bucket);
                    let mut nbucket = bucket;
                    nbucket.push(index);
                    let nbucket = nbucket;
                    rhs_index.insert(k, nbucket);
                }
                None => { rhs_index.insert(k, vec![index]) }
            }
        }

        lhs.retain(|candidate| {
            let k = atom_to_trie_key(candidate);
            let r = rhs_index.get(&k).next();
            match r.cloned() {
                None => { true }
                Some(bucket) => {
                    match bucket.iter().position(|item| &rhs[*item] == candidate) {
                        None => { true }
                        Some(i) => {
                            rhs_index.remove(&k, &bucket);
                            if bucket.len() > 1 {
                                let mut nbucket = bucket;
                                nbucket.remove(i);
                                rhs_index.insert(k, nbucket);
                            }
                            false
                        }
                    }
                }
            }
        });

        Ok(vec![Atom::expr(lhs)])
    }
}

#[derive(Clone, Debug)]
pub struct AddAtomOp {}

grounded_op!(AddAtomOp, "add-atom");

impl Grounded for AddAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, rust_type_atom::<DynSpace>(),
            ATOM_TYPE_ATOM, UNIT_TYPE])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for AddAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("add-atom expects two arguments: space and atom");
        let space = args.get(0).ok_or_else(arg_error)?;
        let atom = args.get(1).ok_or_else(arg_error)?;
        let space = Atom::as_gnd::<DynSpace>(space).ok_or("add-atom expects a space as the first argument")?;
        space.borrow_mut().add(atom.clone());
        unit_result()
    }
}

#[derive(Clone, Debug)]
pub struct RemoveAtomOp {}

grounded_op!(RemoveAtomOp, "remove-atom");

impl Grounded for RemoveAtomOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, rust_type_atom::<DynSpace>(),
            ATOM_TYPE_ATOM, UNIT_TYPE])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for RemoveAtomOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("remove-atom expects two arguments: space and atom");
        let space = args.get(0).ok_or_else(arg_error)?;
        let atom = args.get(1).ok_or_else(arg_error)?;
        let space = Atom::as_gnd::<DynSpace>(space).ok_or("remove-atom expects a space as the first argument")?;
        space.borrow_mut().remove(atom);
        // TODO? Is it necessary to distinguish whether the atom was removed or not?
        unit_result()
    }
}

#[derive(Clone, Debug)]
pub struct GetAtomsOp {}

grounded_op!(GetAtomsOp, "get-atoms");

impl Grounded for GetAtomsOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, rust_type_atom::<DynSpace>(),
            ATOM_TYPE_ATOM])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for GetAtomsOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("get-atoms expects one argument: space");
        let space = args.get(0).ok_or_else(arg_error)?;
        let space = Atom::as_gnd::<DynSpace>(space).ok_or("get-atoms expects a space as its argument")?;
        space.borrow().as_space().atom_iter()
            .map(|iter| iter.cloned().map(|a| make_variables_unique(a)).collect())
            .ok_or(ExecError::Runtime("Unsupported Operation. Can't traverse atoms in this space".to_string()))
    }
}

#[derive(Clone, Debug)]
pub struct GetTypeOp {
    space: DynSpace,
}

grounded_op!(GetTypeOp, "get-type");

impl GetTypeOp {
    pub fn new(space: DynSpace) -> Self {
        Self{ space }
    }
}

impl Grounded for GetTypeOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_ATOM, ATOM_TYPE_ATOM])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for GetTypeOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("get-type expects single atom as an argument");
        let atom = args.get(0).ok_or_else(arg_error)?;
        let space = match args.get(1) {
            Some(space) => Atom::as_gnd::<DynSpace>(space)
                .ok_or("match expects a space as the first argument"),
            None => Ok(&self.space),
        }?;
        let types = get_atom_types(space, atom);
        if types.is_empty() {
            Ok(vec![EMPTY_SYMBOL])
        } else {
            Ok(types)
        }
    }
}

#[derive(Clone, Debug)]
pub struct GetMetaTypeOp { }

grounded_op!(GetMetaTypeOp, "get-metatype");

impl Grounded for GetMetaTypeOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, ATOM_TYPE_ATOM, ATOM_TYPE_ATOM])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for GetMetaTypeOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("get-metatype expects single atom as an argument");
        let atom = args.get(0).ok_or_else(arg_error)?;

        Ok(vec![get_meta_type(&atom)])
    }
}

#[derive(Clone, Debug)]
pub struct GetTypeSpaceOp {}

grounded_op!(GetTypeSpaceOp, "get-type-space");

impl Grounded for GetTypeSpaceOp {
    fn type_(&self) -> Atom {
        Atom::expr([ARROW_SYMBOL, rust_type_atom::<DynSpace>(), ATOM_TYPE_ATOM, ATOM_TYPE_ATOM])
    }

    fn as_execute(&self) -> Option<&dyn CustomExecute> {
        Some(self)
    }
}

impl CustomExecute for GetTypeSpaceOp {
    fn execute(&self, args: &[Atom]) -> Result<Vec<Atom>, ExecError> {
        let arg_error = || ExecError::from("get-type-space expects two arguments: space and atom");
        let space = args.get(0).ok_or_else(arg_error)?;
        let space = Atom::as_gnd::<DynSpace>(space).ok_or("get-type-space expects a space as the first argument")?;
        let atom = args.get(1).ok_or_else(arg_error)?;
        log::debug!("GetTypeSpaceOp::execute: space: {}, atom: {}", space, atom);

        Ok(get_atom_types(space, atom))
    }
}

pub fn register_common_tokens(tref: &mut Tokenizer, _tokenizer: Shared<Tokenizer>, space: &DynSpace, metta: &Metta) {
    let get_type_space_op = Atom::gnd(GetTypeSpaceOp{});
    tref.register_token(regex(r"get-type-space"), move |_| { get_type_space_op.clone() });
    let get_meta_type_op = Atom::gnd(GetMetaTypeOp{});
    tref.register_token(regex(r"get-metatype"), move |_| { get_meta_type_op.clone() });
    let get_type_op = Atom::gnd(GetTypeOp::new(space.clone()));
    tref.register_token(regex(r"get-type"), move |_| { get_type_op.clone() });
    let add_atom_op = Atom::gnd(AddAtomOp{});
    tref.register_token(regex(r"add-atom"), move |_| { add_atom_op.clone() });
    let remove_atom_op = Atom::gnd(RemoveAtomOp{});
    tref.register_token(regex(r"remove-atom"), move |_| { remove_atom_op.clone() });
    let get_atoms_op = Atom::gnd(GetAtomsOp{});
    tref.register_token(regex(r"get-atoms"), move |_| { get_atoms_op.clone() });
    let min_atom_op = Atom::gnd(MinAtomOp{});
    tref.register_token(regex(r"min-atom"), move |_| { min_atom_op.clone() });
    let max_atom_op = Atom::gnd(MaxAtomOp{});
    tref.register_token(regex(r"max-atom"), move |_| { max_atom_op.clone() });
    let size_atom_op = Atom::gnd(SizeAtomOp{});
    tref.register_token(regex(r"size-atom"), move |_| { size_atom_op.clone() });
    let index_atom_op = Atom::gnd(IndexAtomOp{});
    tref.register_token(regex(r"index-atom"), move |_| { index_atom_op.clone() });
    let unique_op = Atom::gnd(UniqueAtomOp{});
    tref.register_token(regex(r"unique-atom"), move |_| { unique_op.clone() });
    let subtraction_op = Atom::gnd(SubtractionAtomOp{});
    tref.register_token(regex(r"subtraction-atom"), move |_| { subtraction_op.clone() });
    let intersection_op = Atom::gnd(IntersectionAtomOp{});
    tref.register_token(regex(r"intersection-atom"), move |_| { intersection_op.clone() });
    let union_op = Atom::gnd(UnionAtomOp{});
    tref.register_token(regex(r"union-atom"), move |_| { union_op.clone() });
    #[cfg(feature = "pkg_mgmt")]
    metta::runner::stdlib::pkg_mgmt_ops::register_pkg_mgmt_tokens(tref, metta);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metta::text::SExprParser;
    use crate::metta::runner::EnvBuilder;
    use crate::metta::runner::string::Str;
    use crate::common::test_utils::metta_space;
    use crate::metta::runner::stdlib::tests::run_program;

    #[test]
    fn metta_car_atom() {
        let result = run_program("!(eval (car-atom (A $b)))");
        assert_eq!(result, Ok(vec![vec![expr!("A")]]));
        let result = run_program("!(eval (car-atom ($a B)))");
        assert_eq!(result, Ok(vec![vec![expr!(a)]]));
        let result = run_program("!(eval (car-atom ()))");
        assert_eq!(result, Ok(vec![vec![expr!("Error" ("car-atom" ()) {Str::from_str("car-atom expects a non-empty expression as an argument")})]]));
        let result = run_program("!(eval (car-atom A))");
        assert_eq!(result, Ok(vec![vec![expr!("Error" ("car-atom" "A") {Str::from_str("car-atom expects a non-empty expression as an argument")})]]));
    }

    #[test]
    fn metta_cdr_atom() {
        assert_eq!(run_program(&format!("!(cdr-atom (a b c))")), Ok(vec![vec![expr!("b" "c")]]));
        assert_eq!(run_program(&format!("!(cdr-atom ($a b $c))")), Ok(vec![vec![expr!("b" c)]]));
        assert_eq!(run_program(&format!("!(cdr-atom ())")), Ok(vec![vec![expr!("Error" ("cdr-atom" ()) {Str::from_str("cdr-atom expects a non-empty expression as an argument")})]]));
        assert_eq!(run_program(&format!("!(cdr-atom a)")), Ok(vec![vec![expr!("Error" ("cdr-atom" "a") {Str::from_str("cdr-atom expects a non-empty expression as an argument")})]]));
        assert_eq!(run_program(&format!("!(cdr-atom $a)")), Ok(vec![vec![expr!("Error" ("cdr-atom" a) {Str::from_str("cdr-atom expects a non-empty expression as an argument")})]]));
    }

    #[test]
    fn metta_size_atom() {
        assert_eq!(run_program(&format!("!(size-atom (5 4 3 2 1))")), Ok(vec![vec![expr!({Number::Integer(5)})]]));
        assert_eq!(run_program(&format!("!(size-atom ())")), Ok(vec![vec![expr!({Number::Integer(0)})]]));
    }

    #[test]
    fn metta_min_atom() {
        assert_eq!(run_program(&format!("!(min-atom (5 4 5.5))")), Ok(vec![vec![expr!({Number::Integer(4)})]]));
        assert_eq!(run_program(&format!("!(min-atom ())")), Ok(vec![vec![expr!("Error" ({ MinAtomOp{} } ()) "Empty expression")]]));
        assert_eq!(run_program(&format!("!(min-atom (3 A B 5))")), Ok(vec![vec![expr!("Error" ({ MinAtomOp{} } ({Number::Integer(3)} "A" "B" {Number::Integer(5)})) "Only numbers are allowed in expression")]]));
    }

    #[test]
    fn metta_max_atom() {
        assert_eq!(run_program(&format!("!(max-atom (5 4 5.5))")), Ok(vec![vec![expr!({Number::Float(5.5)})]]));
        assert_eq!(run_program(&format!("!(max-atom ())")), Ok(vec![vec![expr!("Error" ({ MaxAtomOp{} } ()) "Empty expression")]]));
        assert_eq!(run_program(&format!("!(max-atom (3 A B 5))")), Ok(vec![vec![expr!("Error" ({ MaxAtomOp{} } ({Number::Integer(3)} "A" "B" {Number::Integer(5)})) "Only numbers are allowed in expression")]]));
    }

    #[test]
    fn metta_index_atom() {
        assert_eq!(run_program(&format!("!(index-atom (5 4 3 2 1) 2)")), Ok(vec![vec![expr!({Number::Integer(3)})]]));
        assert_eq!(run_program(&format!("!(index-atom (A B C D E) 5)")), Ok(vec![vec![expr!("Error" ({ IndexAtomOp{} } ("A" "B" "C" "D" "E") {Number::Integer(5)}) "Index is out of bounds")]]));
    }

    #[test]
    fn metta_filter_atom() {
        assert_eq!(run_program("!(eval (filter-atom () $x (eval (if-error $x False True))))"), Ok(vec![vec![expr!()]]));
        assert_eq!(run_program("!(eval (filter-atom (a (b) $c) $x (eval (if-error $x False True))))"), Ok(vec![vec![expr!("a" ("b") c)]]));
        assert_eq!(run_program("!(eval (filter-atom (a (Error (b) \"Test error\") $c) $x (eval (if-error $x False True))))"), Ok(vec![vec![expr!("a" c)]]));
    }

    #[test]
    fn metta_map_atom() {
        assert_eq!(run_program("!(eval (map-atom () $x ($x mapped)))"), Ok(vec![vec![expr!()]]));
        assert_eq!(run_program("!(eval (map-atom (a (b) $c) $x (mapped $x)))"), Ok(vec![vec![expr!(("mapped" "a") ("mapped" ("b")) ("mapped" c))]]));
    }

    #[test]
    fn metta_foldl_atom() {
        assert_eq!(run_program("!(eval (foldl-atom () 1 $a $b (eval (+ $a $b))))"), Ok(vec![vec![expr!({Number::Integer(1)})]]));
        assert_eq!(run_program("!(eval (foldl-atom (1 2 3) 0 $a $b (eval (+ $a $b))))"), Ok(vec![vec![expr!({Number::Integer(6)})]]));
    }

    #[test]
    fn mod_space_op() {
        let program = r#"
            !(bind! &new_space (new-space))
            !(add-atom &new_space (mod-space! stdlib))
            !(get-atoms &new_space)
        "#;
        let runner = Metta::new(Some(runner::environment::EnvBuilder::test_env()));
        let result = runner.run(SExprParser::new(program)).unwrap();

        let stdlib_space = runner.module_space(runner.get_module_by_name("stdlib").unwrap());
        assert_eq!(result[2], vec![Atom::gnd(stdlib_space)]);
    }

    #[test]
    fn size_atom_op() {
        let res = SizeAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} {Number::Integer(3)} {Number::Integer(2)} {Number::Integer(1)})]).expect("No result returned");
        assert_eq!(res, vec![expr!({Number::Integer(5)})]);
        let res = SizeAtomOp{}.execute(&mut vec![expr!()]).expect("No result returned");
        assert_eq!(res, vec![expr!({Number::Integer(0)})]);
    }

    #[test]
    fn min_atom_op() {
        let res = MinAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} {Number::Float(5.5)})]).expect("No result returned");
        assert_eq!(res, vec![expr!({Number::Integer(4)})]);
        let res = MinAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} "A")]);
        assert_eq!(res, Err(ExecError::from("Only numbers are allowed in expression")));
        let res = MinAtomOp{}.execute(&mut vec![expr!()]);
        assert_eq!(res, Err(ExecError::from("Empty expression")));
    }

    #[test]
    fn max_atom_op() {
        let res = MaxAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} {Number::Float(5.5)})]).expect("No result returned");
        assert_eq!(res, vec![expr!({Number::Float(5.5)})]);
        let res = MaxAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} "A")]);
        assert_eq!(res, Err(ExecError::from("Only numbers are allowed in expression")));
        let res = MaxAtomOp{}.execute(&mut vec![expr!()]);
        assert_eq!(res, Err(ExecError::from("Empty expression")));
    }

    #[test]
    fn index_atom_op() {
        let res = IndexAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} {Number::Integer(3)} {Number::Integer(2)} {Number::Integer(1)}), expr!({Number::Integer(2)})]).expect("No result returned");
        assert_eq!(res, vec![expr!({Number::Integer(3)})]);
        let res = IndexAtomOp{}.execute(&mut vec![expr!({Number::Integer(5)} {Number::Integer(4)} {Number::Integer(3)} {Number::Integer(2)} {Number::Integer(1)}), expr!({Number::Integer(5)})]);
        assert_eq!(res, Err(ExecError::from("Index is out of bounds")));
    }

    #[test]
    fn add_atom_op() {
        let space = DynSpace::new(GroundingSpace::new());
        let satom = Atom::gnd(space.clone());
        let res = AddAtomOp{}.execute(&mut vec![satom, expr!(("foo" "bar"))]).expect("No result returned");
        assert_eq!(res, vec![UNIT_ATOM]);
        let space_atoms: Vec<Atom> = space.borrow().as_space().atom_iter().unwrap().cloned().collect();
        assert_eq_no_order!(space_atoms, vec![expr!(("foo" "bar"))]);
    }

    #[test]
    fn test_error_is_used_as_an_argument() {
        let metta = Metta::new(Some(EnvBuilder::test_env()));
        let parser = SExprParser::new(r#"
            !(get-type Error)
            !(get-metatype Error)
            !(get-type (Error Foo Boo))
            !(Error (+ 1 2) (+ 1 +))
        "#);

        assert_eq_metta_results!(metta.run(parser), Ok(vec![
            vec![expr!("->" "Atom" "Atom" "ErrorType")],
            vec![expr!("Symbol")],
            vec![expr!("ErrorType")],
            vec![expr!("Error" ({SumOp{}} {Number::Integer(1)} {Number::Integer(2)}) ({SumOp{}} {Number::Integer(1)} {SumOp{}}))],
        ]));
    }

    #[test]
    fn remove_atom_op() {
        let space = DynSpace::new(metta_space("
            (foo bar)
            (bar foo)
        "));
        let satom = Atom::gnd(space.clone());
        let res = RemoveAtomOp{}.execute(&mut vec![satom, expr!(("foo" "bar"))]).expect("No result returned");
        // REM: can return Bool in future
        assert_eq!(res, vec![UNIT_ATOM]);
        let space_atoms: Vec<Atom> = space.borrow().as_space().atom_iter().unwrap().cloned().collect();
        assert_eq_no_order!(space_atoms, vec![expr!(("bar" "foo"))]);
    }

    #[test]
    fn get_atoms_op() {
        let space = DynSpace::new(metta_space("
            (foo bar)
            (bar foo)
        "));
        let satom = Atom::gnd(space.clone());
        let res = GetAtomsOp{}.execute(&mut vec![satom]).expect("No result returned");
        let space_atoms: Vec<Atom> = space.borrow().as_space().atom_iter().unwrap().cloned().collect();
        assert_eq_no_order!(res, space_atoms);
        assert_eq_no_order!(res, vec![expr!(("foo" "bar")), expr!(("bar" "foo"))]);
    }

    #[test]
    fn unique_op() {
        let unique_op = UniqueAtomOp{};
        let actual = unique_op.execute(&mut vec![expr!(
                ("A" ("B" "C"))
                ("A" ("B" "C"))
                ("f" "g")
                ("f" "g")
                ("f" "g")
                "Z"
        )]).unwrap();
        assert_eq_no_order!(actual,
                   vec![expr!(("A" ("B" "C")) ("f" "g") "Z")]);
    }

    #[test]
    fn union_op() {
        let union_op = UnionAtomOp{};
        let actual = union_op.execute(&mut vec![expr!(
                ("A" ("B" "C"))
                ("A" ("B" "C"))
                ("f" "g")
                ("f" "g")
                ("f" "g")
                "Z"
            ), expr!(
                ("A" ("B" "C"))
                "p"
                "p"
                ("Q" "a")
            )]).unwrap();
        assert_eq_no_order!(actual,
                   vec![expr!(("A" ("B" "C")) ("A" ("B" "C"))
                        ("f" "g") ("f" "g") ("f" "g") "Z"
                        ("A" ("B" "C")) "p" "p" ("Q" "a"))]);
    }

    #[test]
    fn intersection_op() {
        let intersection_op = IntersectionAtomOp{};
        let actual = intersection_op.execute(&mut vec![expr!(
                "Z"
                ("A" ("B" "C"))
                ("A" ("B" "C"))
                ("f" "g")
                ("f" "g")
                ("f" "g")
                ("P" "b")
            ), expr!(
                ("f" "g")
                ("f" "g")
                ("A" ("B" "C"))
                "p"
                "p"
                ("Q" "a")
                "Z"
            )]).unwrap();
        assert_eq_no_order!(actual, vec![expr!("Z" ("A" ("B" "C")) ("f" "g") ("f" "g"))]);

        assert_eq_no_order!(intersection_op.execute(&mut vec![expr!(
                { Number::Integer(5) }
                { Number::Integer(4) }
                { Number::Integer(3) }
                { Number::Integer(2) }
            ), expr!(
                { Number::Integer(5) }
                { Number::Integer(3) }
            )]).unwrap(), vec![expr!({Number::Integer(5)} {Number::Integer(3)})]);
    }

    #[test]
    fn subtraction_op() {
        let subtraction_op = SubtractionAtomOp{};
        let actual = subtraction_op.execute(&mut vec![expr!(
                "Z"
                "S"
                "S"
                ("A" ("B" "C"))
                ("A" ("B" "C"))
                ("f" "g")
                ("f" "g")
                ("f" "g")
                ("P" "b")
            ), expr!(
                ("f" "g")
                ("A" ("B" "C"))
                "p"
                "P"
                ("Q" "a")
                "Z"
                "S"
                "S"
                "S"
            )]).unwrap();
        assert_eq_no_order!(actual,
                   vec![expr!(("A" ("B" "C")) ("f" "g") ("f" "g") ("P" "b"))]);
    }

    #[test]
    fn get_type_op() {
        let space = DynSpace::new(metta_space("
            (: B Type)
            (: C Type)
            (: A B)
            (: A C)
        "));

        let get_type_op = GetTypeOp::new(space.clone());
        assert_eq_no_order!(get_type_op.execute(&mut vec![sym!("A"), expr!({space.clone()})]).unwrap(),
            vec![sym!("B"), sym!("C")]);
    }

    #[test]
    fn get_type_op_non_valid_atom() {
        let space = DynSpace::new(metta_space("
            (: f (-> Number String))
            (: 42 Number)
            (: \"test\" String)
        "));

        let get_type_op = GetTypeOp::new(space.clone());
        assert_eq_no_order!(get_type_op.execute(&mut vec![expr!("f" "42"), expr!({space.clone()})]).unwrap(),
            vec![sym!("String")]);
        assert_eq_no_order!(get_type_op.execute(&mut vec![expr!("f" "\"test\""), expr!({space.clone()})]).unwrap(),
            vec![EMPTY_SYMBOL]);
    }
}