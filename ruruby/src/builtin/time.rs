use crate::*;
use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Utc};

#[derive(Clone, Debug, PartialEq)]
pub enum TimeInfo {
    Local(DateTime<FixedOffset>),
    UTC(DateTime<Utc>),
}

impl std::fmt::Display for TimeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeInfo::Local(t) => write!(f, "{}", t),
            TimeInfo::UTC(t) => write!(f, "{}", t),
        }
    }
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

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("Time", class);
    class.add_builtin_class_method(globals, "now", time_now);
    class.add_builtin_class_method(globals, "utc", time_utc);
    class.add_builtin_class_method(globals, "gm", time_utc);

    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "to_s", to_s);
    class.add_builtin_method_by_str(globals, "gmtime", utc);
    class.add_builtin_method_by_str(globals, "utc", utc);
    class.add_builtin_method_by_str(globals, "-", sub);
    class.add_builtin_method_by_str(globals, "+", add);
    class.add_builtin_method_by_str(globals, "year", year);
    class.add_builtin_method_by_str(globals, "month", month);
    class.add_builtin_method_by_str(globals, "mon", month);
    class.add_builtin_method_by_str(globals, "mday", day);
    class.add_builtin_method_by_str(globals, "day", day);
    class.into()
}

fn time_now(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let t = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
    let time_info = TimeInfo::Local(t);
    let new_obj = Value::time(Module::new(self_val), time_info);
    Ok(new_obj)
}

/// Time.gm(year, mon = 1, day = 1, hour = 0, min = 0, sec = 0, usec = 0) -> time
/// Time.utc(year, mon = 1, day = 1, hour = 0, min = 0, sec = 0, usec = 0) -> time
/// https://docs.ruby-lang.org/ja/latest/method/Time/s/gm.html
fn time_utc(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(1, 8)?;
    let (y, m, d, h, min, sec, usec) = match args.len() {
        1 => (vm[0].coerce_to_fixnum("Args")?, 1, 1, 0, 0, 0, 0),
        2 => (
            vm[0].coerce_to_fixnum("Args")?,
            vm[1].coerce_to_fixnum("Args")?,
            1,
            0,
            0,
            0,
            0,
        ),
        3 => (
            vm[0].coerce_to_fixnum("Args")?,
            vm[1].coerce_to_fixnum("Args")?,
            vm[2].coerce_to_fixnum("Args")?,
            0,
            0,
            0,
            0,
        ),
        4 => (
            vm[0].coerce_to_fixnum("Args")?,
            vm[1].coerce_to_fixnum("Args")?,
            vm[2].coerce_to_fixnum("Args")?,
            vm[3].coerce_to_fixnum("Args")?,
            0,
            0,
            0,
        ),
        5 => (
            vm[0].coerce_to_fixnum("Args")?,
            vm[1].coerce_to_fixnum("Args")?,
            vm[2].coerce_to_fixnum("Args")?,
            vm[3].coerce_to_fixnum("Args")?,
            vm[4].coerce_to_fixnum("Args")?,
            0,
            0,
        ),
        6 => (
            vm[0].coerce_to_fixnum("Args")?,
            vm[1].coerce_to_fixnum("Args")?,
            vm[2].coerce_to_fixnum("Args")?,
            vm[3].coerce_to_fixnum("Args")?,
            vm[4].coerce_to_fixnum("Args")?,
            vm[5].coerce_to_fixnum("Args")?,
            0,
        ),
        7 => (
            vm[0].coerce_to_fixnum("Args")?,
            vm[1].coerce_to_fixnum("Args")?,
            vm[2].coerce_to_fixnum("Args")?,
            vm[3].coerce_to_fixnum("Args")?,
            vm[4].coerce_to_fixnum("Args")?,
            vm[5].coerce_to_fixnum("Args")?,
            vm[6].coerce_to_fixnum("Args")?,
        ),
        _ => unreachable!(),
    };
    let native_dt = NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32)
        .ok_or_else(|| RubyError::argument("Out of range."))?
        .and_hms_micro_opt(h as u32, min as u32, sec as u32, usec as u32)
        .ok_or_else(|| RubyError::argument("Out of range."))?;
    let time = TimeInfo::UTC(DateTime::<Utc>::from_utc(native_dt, Utc));
    Ok(Value::time(Module::new(self_val), time))
}

/// Time#inspect -> String
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/inspect.html
fn inspect(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let time = self_val.as_time();
    Ok(Value::string(format!("{}", time)))
}

/// Time#to_s -> String
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/to_s.html
fn to_s(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let time = self_val.as_time();
    let s = match time {
        TimeInfo::Local(t) => format!("{}", t.format("%Y-%m-%d %H:%M:%S %z")),
        TimeInfo::UTC(t) => format!("{}", t.format("%Y-%m-%d %H:%M:%S UTC")),
    };
    Ok(Value::string(s))
}

/// TIme#gmtime -> self
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/gmtime.html
fn utc(vm: &mut VM, mut self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let time = self_val.as_mut_time();
    match time {
        TimeInfo::Local(t) => *time = TimeInfo::UTC(t.clone().into()),
        TimeInfo::UTC(_) => {}
    };
    Ok(self_val)
}

/// self - time -> Float
/// self - sec -> Time
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/=2d.html
fn sub(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let time = self_val.as_time().clone();
    let arg0 = vm[0];
    match arg0.unpack() {
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
            _ => return Err(VMError::undefined_op("-", arg0, self_val)),
        },
        _ => return Err(VMError::undefined_op("-", arg0, self_val)),
    }
}

/// self + other -> Time
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/=2b.html
fn add(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let time = match &self_val.rvalue().kind {
        ObjKind::Time(time) => (**time).clone(),
        _ => unreachable!(),
    };
    let arg0 = vm[0];
    match arg0.unpack() {
        RV::Integer(i) => {
            let res = time + Duration::seconds(i);
            Ok(Value::time(self_val.get_class(), res))
        }
        RV::Float(f) => {
            let offset = (f * 1000.0 * 1000.0 * 1000.0) as i64;
            let res = time + Duration::nanoseconds(offset);
            Ok(Value::time(self_val.get_class(), res))
        }
        _ => return Err(VMError::undefined_op("+", arg0, self_val)),
    }
}

/// Time#year -> Integer
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/year.html
fn year(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let num = match self_val.as_time() {
        &TimeInfo::Local(t) => t.year(),
        &TimeInfo::UTC(t) => t.year(),
    };
    Ok(Value::integer(num as i64))
}

/// Time#mon -> Integer
/// Time#month -> Integer
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/mon.html
fn month(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let num = match self_val.as_time() {
        &TimeInfo::Local(t) => t.month(),
        &TimeInfo::UTC(t) => t.month(),
    };
    Ok(Value::integer(num as i64))
}

/// Time#mday -> Integer
/// Time#day -> Integer
/// https://docs.ruby-lang.org/ja/latest/method/Time/i/day.html
fn day(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let num = match self_val.as_time() {
        &TimeInfo::Local(t) => t.day(),
        &TimeInfo::UTC(t) => t.day(),
    };
    Ok(Value::integer(num as i64))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn time() {
        let program = r#"
        t = Time.utc(1,2,3,4,5,6,7)
        assert "0001-02-03 04:05:06.000007 UTC", t.inspect
        assert "0001-02-03 04:05:06 UTC", t.to_s
        assert "0001-02-03 04:00:00 UTC", Time.gm(1,2,3,4).inspect
        assert "0001-02-03 04:00:00 UTC", Time.gm(1,2,3,4).to_s
        assert 1, t.year
        assert 2, t.month
        assert 2, t.mon
        assert 3, t.day
        assert 3, t.mday
        assert t.gmtime.inspect, t.utc.inspect
    "#;
        assert_script(program);
    }

    #[test]
    fn time_ops() {
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
