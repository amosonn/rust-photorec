
#[macro_export]
macro_rules! assert_let {
    ($lhs:pat = $rhs:expr) => {
        if let $lhs = $rhs {} else { panic!() }
    };
    //($lhs:pat = $rhs:expr, $stmt:stmt) => {
        //if let $lhs = $rhs { $stmt } else { panic!() }
    //};
    ($lhs:pat = $rhs:expr, $code:block) => {
        if let $lhs = $rhs $code else { panic!() }
    };
}
