use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let class = Value::class_under(globals.builtins.object);
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("clear", clear);
    class.add_builtin_method_by_str("clone", clone);
    class.add_builtin_method_by_str("dup", clone);
    class.add_builtin_method_by_str("compact", compact);
    class.add_builtin_method_by_str("delete", delete);
    class.add_builtin_method_by_str("empty?", empty);
    class.add_builtin_method_by_str("select", select);
    class.add_builtin_method_by_str("has_key?", has_key);
    class.add_builtin_method_by_str("key?", has_key);
    class.add_builtin_method_by_str("include?", has_key);
    class.add_builtin_method_by_str("member?", has_key);
    class.add_builtin_method_by_str("has_value?", has_value);
    class.add_builtin_method_by_str("keys", keys);
    class.add_builtin_method_by_str("length", length);
    class.add_builtin_method_by_str("size", length);
    class.add_builtin_method_by_str("values", values);
    class.add_builtin_method_by_str("each_value", each_value);
    class.add_builtin_method_by_str("each_key", each_key);
    class.add_builtin_method_by_str("each", each);
    class.add_builtin_method_by_str("merge", merge);
    class.add_builtin_method_by_str("fetch", fetch);
    class.add_builtin_method_by_str("compare_by_identity", compare_by_identity);
    class.add_builtin_method_by_str("sort", sort);
    class.add_builtin_method_by_str("invert", invert);

    class.add_builtin_class_method("new", hash_new);
    *class
}

// Class methods

fn hash_new(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let map = FxHashMap::default();
    let hash = Value::hash_from_map(map);
    Ok(hash)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let s = hash.to_s(vm)?;
    Ok(Value::string(s))
}

fn clear(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_mut_hash().unwrap();
    hash.clear();
    Ok(self_val)
}

fn clone(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::hash_from(hash.clone()))
}

fn compact(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut hash = self_val.expect_hash("Receiver")?.clone();
    match hash {
        HashInfo::Map(ref mut map) => map.retain(|_, &mut v| v != Value::nil()),
        HashInfo::IdentMap(ref mut map) => map.retain(|_, &mut v| v != Value::nil()),
    };
    Ok(Value::hash_from(hash))
}

fn delete(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let hash = self_val.as_mut_hash().unwrap();
    let res = match hash.remove(args[0]) {
        Some(v) => v,
        None => Value::nil(),
    };
    Ok(res)
}

fn empty(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::bool(hash.len() == 0))
}

fn select(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let hash = self_val.as_hash().unwrap();
    let method = args.expect_block()?;
    let mut res = FxHashMap::default();
    let mut arg = Args::new(2);
    for (k, v) in hash.iter() {
        arg[0] = k;
        arg[1] = v;
        if vm.eval_block(&method, &arg)?.to_bool() {
            res.insert(HashKey(k), v);
        };
    }

    Ok(Value::hash_from_map(res))
}

fn has_key(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let hash = self_val.as_hash().unwrap();
    let res = hash.contains_key(args[0]);
    Ok(Value::bool(res))
}

fn has_value(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let hash = self_val.as_hash().unwrap();
    let res = hash.iter().find(|(_, v)| *v == args[0]).is_some();
    Ok(Value::bool(res))
}

fn length(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let len = hash.len();
    Ok(Value::integer(len as i64))
}

fn keys(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::array_from(hash.keys()))
}

fn values(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    Ok(Value::array_from(hash.values()))
}

fn each_value(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let block = args.expect_block()?;
    let mut arg = Args::new(1);
    for (_, v) in hash.iter() {
        arg[0] = v;
        vm.eval_block(&block, &arg)?;
    }

    Ok(self_val)
}

fn each_key(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let block = args.expect_block()?;
    let mut arg = Args::new(1);

    for (k, _) in hash.iter() {
        arg[0] = k;
        vm.eval_block(&block, &arg)?;
    }

    Ok(self_val)
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let block = args.expect_block()?;
    let mut arg = Args::new(2);

    for (k, v) in hash.iter() {
        arg[0] = k;
        arg[1] = v;
        vm.eval_block(&block, &arg)?;
    }

    Ok(self_val)
}

fn merge(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut new = (self_val.expect_hash("Receiver")?).clone();
    for arg in args.iter() {
        let other = arg.expect_hash("First arg")?;
        for (k, v) in other.iter() {
            new.insert(k, v);
        }
    }

    Ok(Value::hash_from(new))
}

fn fetch(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let key = args[0];

    let hash = self_val.as_hash().unwrap();
    let val = match hash.get(&key) {
        Some(val) => *val,
        None => {
            match &args.block {
                // TODO: If arg[1] exists, Should warn "block supersedes default value argument".
                Block::None => {
                    if args.len() == 2 {
                        args[1]
                    } else {
                        // TODO: Should be KeyError.
                        return Err(RubyError::argument("Key not found."));
                    }
                }
                block => vm.eval_block(block, &Args::new1(key))?,
            }
        }
    };

    Ok(val)
}

fn compare_by_identity(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_mut_hash().unwrap();
    match hash {
        HashInfo::Map(map) => {
            let new_map = map.into_iter().map(|(k, v)| (IdentKey(k.0), *v)).collect();
            *hash = HashInfo::IdentMap(new_map);
        }
        HashInfo::IdentMap(_) => {}
    };
    Ok(self_val)
}

fn sort(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let mut vec = vec![];
    for (k, v) in hash.iter() {
        let ary = vec![k, v];
        vec.push(Value::array_from(ary));
    }
    vm.sort_array(&mut vec)?;
    Ok(Value::array_from(vec))
}

fn invert(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let hash = self_val.as_hash().unwrap();
    let mut new_hash = FxHashMap::default();
    for (k, v) in hash.iter() {
        new_hash.insert(HashKey(v), k);
    }
    Ok(Value::hash_from_map(new_hash))
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn hash_new() {
        let program = r#"
            assert ({}), Hash.new
            "#;
        assert_script(program);
    }

    #[test]
    fn hash1() {
        let program = r#"
            h = {true => "true", false => "false", nil => "nil", 100 => "100", 7.7 => "7.7",
            "ruby" => "string", :ruby => "symbol", [1,2,3] => {a:1}, {b:3} => [3,4,5], 1..4 => "1"}
            assert(h[true], "true")
            assert(h[false], "false")
            assert(h[nil], "nil")
            assert(h[100], "100")
            assert(h[7.7], "7.7")
            assert(h["ruby"], "string")
            assert(h[:ruby], "symbol")
            assert(h[[1,2,3]], {a:1})
            assert(h[{b:3}], [3,4,5])
            assert(h[1..4], "1")
            assert([], h.keys - [true, false, nil, 100, 7.7, "ruby", :ruby, [1,2,3], {b:3}, 1..4])
            assert([], h.values - ["true", "false", "nil", "100", "7.7", "string", "symbol", {a:1}, [3,4,5], "1"])

            {a:7} == eval({a:7}.to_s)

            h = {true => "true", false => "false", nil => "nil", 100 => "100", 7.7 => "7.7",
            "ruby" => "string", :ruby => "symbol", [1,2,3] => {a:1}, {b:3} => [3,4,5], 1..4 => "1"}.compare_by_identity
            assert(h[true], "true")
            assert(h[false], "false")
            assert(h[nil], "nil")
            assert(h[100], "100")
            assert(h[7.7], "7.7")
            assert(false, h["ruby"]=="string")
            assert(h[:ruby], "symbol")
            assert(false, h[[1,2,3]]=={a:1})
            assert(false, h[{b:3}]==[3,4,5])
            assert(false, h[1..4]=="1")
            assert([], h.keys - [true, false, nil, 100, 7.7, "ruby", :ruby, [1,2,3], {b:3}, 1..4])
            assert([], h.values - ["true", "false", "nil", "100", "7.7", "string", "symbol", {a:1}, [3,4,5], "1"])
        "#;
        assert_script(program);
    }

    #[test]
    fn hash2() {
        let program = r#"
            a = "100"
            @b = 7.7
            h = {true: "true", false: "false", nil: "nil", 100 => a, @b => "7.7", ruby: "string"}
            assert(h[:true], "true")
            assert(h[:false], "false")
            assert(h[:nil], "nil")
            assert(h[100], "100")
            assert(h[7.7], "7.7")
            assert(h[:ruby], "string")
            
            h2 = {true: "true", false: "false", nil: "nil", 100 => a, @b => "7.7", ruby: "string"}.compare_by_identity
            assert(h2[:true], "true")
            assert(h2[:false], "false")
            assert(h2[:nil], "nil")
            assert(h2[100], "100")
            assert(h2[7.7], "7.7")
            assert(h2[:ruby], "string")

            a = []
            h.each_key{|k| a << k}
            assert([], a - [:true, :false, :nil, 100, 7.7, :ruby])
            a = []
            h.each_value{|v| a << v}
            assert([], a - ["true", "false", "nil", "100", "7.7", "string"])
            a = []
            h.each{|k, v| a << [k, v];}
            assert([], a - [[:true, "true"], [:false, "false"], [:nil, "nil"], [100, "100"], [7.7, "7.7"], [:ruby, "string"]])
            a = []
            h2.each_key{|k| a << k}
            assert([], a - [:true, :false, :nil, 100, 7.7, :ruby])
            a = []
            h2.each_value{|v| a << v}
            assert([], a - ["true", "false", "nil", "100", "7.7", "string"])
            a = []
            h2.each{|k, v| a << [k, v];}
            assert([], a - [[:true, "true"], [:false, "false"], [:nil, "nil"], [100, "100"], [7.7, "7.7"], [:ruby, "string"]])
        "#;
        assert_script(program);
    }

    #[test]
    fn hash3() {
        let program = r#"
            h1 = {a: "symbol", c:nil, d:nil}
            assert(h1.has_key?(:a), true)
            assert(h1.has_key?(:b), false)
            assert(h1.has_value?("symbol"), true)
            assert(h1.has_value?(500), false)
            assert(h1.length, 3)
            assert(h1.size, 3)
            assert([], h1.keys - [:a, :d, :c])
            assert([], h1.values - ["symbol", nil, nil])
            h2 = h1.clone()
            h2[:b] = 100
            assert(h2[:b], 100)
            assert(h1[:b], nil)
            h3 = h2.compact
            assert(h3.delete(:a), "symbol")
            assert(h3.empty?, false)
            assert(h3.delete(:b), 100)
            assert(h3.delete(:c), nil)
            assert(h3.empty?, true)
            h2.clear()
            assert(h2.empty?, true)

            h1 = {a: "symbol", c:nil, d:nil}.compare_by_identity
            assert(h1.has_key?(:a), true)
            assert(h1.has_key?(:b), false)
            assert(h1.has_value?("symbol"), true)
            assert(h1.has_value?(500), false)
            assert(h1.length, 3)
            assert(h1.size, 3)
            assert([], h1.keys - [:a, :d, :c])
            assert([], h1.values - ["symbol", nil, nil])
            h2 = h1.clone()
            h2[:b] = 100
            assert(h2[:b], 100)
            assert(h1[:b], nil)
            h3 = h2.compact
            assert(h3.delete(:a), "symbol")
            assert(h3.empty?, false)
            assert(h3.delete(:b), 100)
            assert(h3.delete(:c), nil)
            assert(h3.empty?, true)
            h2.clear()
            assert(h2.empty?, true)
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_select() {
        let program = r#"
            h = { "a" => 100, "b" => 200, "c" => 300 }
            assert({"b" => 200, "c" => 300}, h.select {|k,v| k > "a"})  #=> {"b" => 200, "c" => 300}
            assert({"a" => 100}, h.select {|k,v| v < 200})  #=> {"a" => 100}

            h = { "a" => 100, "b" => 200, "c" => 300 }.compare_by_identity
            assert({"b" => 200, "c" => 300}, h.select {|k,v| k > "a"})  #=> {"b" => 200, "c" => 300}
            assert({"a" => 100}, h.select {|k,v| v < 200})  #=> {"a" => 100}
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_merge1() {
        let program = r#"
        h1 = { "a" => 100, "b" => 200 }
        h2 = { "b" => 246, "c" => 300 }
        h3 = { "b" => 357, "d" => 400 }
        assert({"a"=>100, "b"=>200}, h1.merge)
        assert({"a"=>100, "b"=>246, "c"=>300}, h1.merge(h2)) 
        assert({"a"=>100, "b"=>357, "c"=>300, "d"=>400}, h1.merge(h2, h3)) 
        assert({"a"=>100, "b"=>200}, h1)
    "#;
        assert_script(program);
    }

    #[test]
    fn hash_merge2() {
        let program = r#"
        h1 = { "a" => 100, "b" => 200 }.compare_by_identity
        h2 = { "b" => 246, "c" => 300 }.compare_by_identity
        h3 = { "b" => 357, "d" => 400 }.compare_by_identity
        assert({"a"=>100, "b"=>200}.compare_by_identity, h1.merge)
        r1 = {}.compare_by_identity
        r1["a"] = 100
        r1["b"] = 200
        r1["b"] = 246
        r1["c"] = 300
        assert(r1, h1.merge(h2)) 
        r1 = {}.compare_by_identity
        r1["a"] = 100
        r1["b"] = 200
        r1["b"] = 246
        r1["b"] = 357
        r1["c"] = 300
        r1["d"] = 400
        assert(r1, h1.merge(h2, h3)) 
        assert({"a"=>100, "b"=>200}.compare_by_identity, h1)
    "#;
        assert_script(program);
    }

    #[test]
    fn hash_compare_by_identity() {
        let program = r#"
        a = "a"
        h1 = {}
        h1[a] = 100
        assert 100, h1["a"]
        assert 100, h1[a]
        h2 = {}
        h2.compare_by_identity
        h2[a] = 100
        assert nil, h2["a"]
        assert 100, h2[a]
    "#;
        assert_script(program);
    }

    #[test]
    fn hash_sort() {
        let program = r#"
        h = { 0 => 20, 1 => 30, 2 => 10  }
        assert([[0, 20], [1, 30], [2, 10]], h.sort)

        h = { 0 => 20, 1 => 30, 2 => 10  }.compare_by_identity
        assert([[0, 20], [1, 30], [2, 10]], h.sort)
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_invert() {
        let program = r#"
        h = { "a" => 0, "b" => 100, "c" => 200, "e" => 300 }
        assert({0=>"a", 100=>"b", 200=>"c", 300=>"e"}, h.invert)

        h = { "a" => 0, "b" => 100, "c" => 200, "e" => 300 }.compare_by_identity
        assert({0=>"a", 100=>"b", 200=>"c", 300=>"e"}, h.invert)
        "#;
        assert_script(program);
    }

    #[test]
    fn hash_fetch() {
        let program = r##"
            h = {one: nil}
            assert(nil, h[:one])                    #=> nil これではキーが存在するのか判別できない。
            assert(nil, h[:two])                    #=> nil これではキーが存在するのか判別できない。
            assert(nil, h.fetch(:one))
            assert_error { h.fetch(:two) }          # エラー key not found (KeyError)
            assert("error", h.fetch(:two,"error"))
            assert("two not exist", h.fetch(:two) {|key|"#{key} not exist"})
            res = h.fetch(:two, "error"){|key|
                "#{key} not exist"                  #  warning: block supersedes default value argument
            }        
            assert("two not exist", res)
            #h.default = "default"
            assert_error { h.fetch(:two) }          # エラー key not found (KeyError)
        "##;
        assert_script(program);
    }
}
