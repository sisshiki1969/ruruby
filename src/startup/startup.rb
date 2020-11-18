class Signal
  def self.trap(type)
  end
end

class Enumerable
end

class RbConfig
  SIZEOF = {"int"=>4, "short"=>2, "long"=>8, "long long"=>8, "__int128"=>16, "off_t"=>8, "void*"=>8, "float"=>4, "double"=>8, "time_t"=>8, "clock_t"=>8, "size_t"=>8, "ptrdiff_t"=>8, "int8_t"=>1, "uint8_t"=>1, "int16_t"=>2, "uint16_t"=>2, "int32_t"=>4, "uint32_t"=>4, "int64_t"=>8, "uint64_t"=>8, "int128_t"=>16, "uint128_t"=>16, "intptr_t"=>8, "uintptr_t"=>8, "ssize_t"=>8, "int_least8_t"=>1, "int_least16_t"=>2, "int_least32_t"=>4, "int_least64_t"=>8, "int_fast8_t"=>1, "int_fast16_t"=>8, "int_fast32_t"=>8, "int_fast64_t"=>8, "intmax_t"=>8, "sig_atomic_t"=>4, "wchar_t"=>4, "wint_t"=>4, "wctrans_t"=>8, "wctype_t"=>8, "_Bool"=>1, "long double"=>16, "float _Complex"=>8, "double _Complex"=>16, "long double _Complex"=>32, "__float128"=>16, "_Decimal32"=>4, "_Decimal64"=>8, "_Decimal128"=>16, "__float80"=>16}
end

class Thread
  @@current = {}.compare_by_identity
  def respond_to?(*x)
    false
  end
  def self.current
    @@current
  end
end

class Delegator
end

class RubyVM
  class AbstractSyntaxTree
    class Node
    end
  end
end

class Symbol
end

class TrueClass
end

class FalseClass
end

class NilClass
end

class Module
  def undef_method(sym); end
  def define_method(sym); end
end

RUBY_PLATFORM = "x86_64-linux"
RUBY_VERSION = "2.5.0"
RUBY_ENGINE = "ruruby"
RUBY_DESCRIPTION = "ruruby [x86_64-linux]"
exec_path = ".rbenv/versions/3.0.0-dev/lib/ruby"
version = "3.0.0"
$: = ["#{Dir.home}/.rbenv/rbenv.d/exec/gem-rehash",
  "#{Dir.home}/#{exec_path}/site_ruby/#{version}",
  "#{Dir.home}/#{exec_path}/site_ruby/#{version}/#{RUBY_PLATFORM}",
  "#{Dir.home}/#{exec_path}/site_ruby",
  "#{Dir.home}/#{exec_path}/vendor_ruby/#{version}",
  "#{Dir.home}/#{exec_path}/vendor_ruby/#{version}/#{RUBY_PLATFORM}",
  "#{Dir.home}/#{exec_path}/vendor_ruby",
  "#{Dir.home}/#{exec_path}/#{version}",
  "#{Dir.home}/#{exec_path}/#{version}/#{RUBY_PLATFORM}"]