use crate::*;
use chrono::{DateTime, Duration, FixedOffset, Utc};

#[derive(Clone, Debug, PartialEq)]
pub enum TimeInfo {
    Local(DateTime<FixedOffset>),
    UTC(DateTime<Utc>),
}

impl std::ops::Sub<Self> for TimeInfo {
    type Output = Duration;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (TimeInfo::Local(t), TimeInfo::Local(rhs)) => t - rhs,
            (TimeInfo::Local(t), TimeInfo::UTC(rhs)) => t.with_timezone(&Utc) - rhs,
            (TimeInfo::UTC(t), TimeInfo::Local(rhs)) => t - rhs.with_timezone(&Utc),
            (TimeInfo::UTC(t), TimeInfo::UTC(rhs)) => t - rhs,
        }
    }
}

impl std::ops::Sub<Duration> for TimeInfo {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self::Output {
        match self {
            TimeInfo::Local(t) => Self::Local(t - rhs),
            TimeInfo::UTC(t) => Self::UTC(t - rhs),
        }
    }
}

impl std::ops::Add<Duration> for TimeInfo {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self::Output {
        match self {
            TimeInfo::Local(t) => Self::Local(t + rhs),
            TimeInfo::UTC(t) => Self::UTC(t + rhs),
        }
    }
}

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Time", class);
    class.add_builtin_class_method("now", time_now);

    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("+", add);
    class.into()
}

fn time_now(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let t = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
    let time_info = TimeInfo::Local(t);
    let new_obj = Value::time(Module::new(self_val), time_info);
    Ok(new_obj)
}

fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => (**time).clone(),
        _ => unreachable!(),
    };
    Ok(Value::string(format!("{:?}", time)))
}

fn sub(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => (**time).clone(),
        _ => unreachable!(),
    };
    match args[0].unpack() {
        RV::Integer(i) => {
            let res = time - Duration::seconds(i);
            Ok(Value::time(self_val.get_class(), res))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time - Duration::nanoseconds(offset);
            Ok(Value::time(self_val.get_class(), res))
        }
        RV::Object(rv) => match &rv.kind {
            ObjKind::Time(t) => {
                let res = time - (**t).clone();
                let offset = (res.num_nanoseconds().unwrap() as f64) / 1000.0 / 1000.0 / 1000.0;
                Ok(Value::float(offset))
            }
            _ => return Err(RubyError::undefined_op("-", args[0], self_val)),
        },
        _ => return Err(RubyError::undefined_op("-", args[0], self_val)),
    }
}

fn add(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => (**time).clone(),
        _ => unreachable!(),
    };
    match args[0].unpack() {
        RV::Integer(i) => {
            let res = time + Duration::seconds(i);
            Ok(Value::time(self_val.get_class(), res))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time + Duration::nanoseconds(offset);
            Ok(Value::time(self_val.get_class(), res))
        }
        _ => return Err(RubyError::undefined_op("+", args[0], self_val)),
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn time() {
        let program = "
        p Time.now.inspect
        a = Time.now
        assert a, a - 100 + 100
        assert a, a - 77.0 + 77.0
        assert Float, (Time.now - a).class
        assert_error { Time.now + a }
    ";
        assert_script(program);
    }
}
