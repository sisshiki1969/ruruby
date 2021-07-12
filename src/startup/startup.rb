class Signal
  def self.trap(type)
  end
end

module Enumerable
end

module Marshal
  MAJOR_VERSION = 0
  MINOR_VERSION = 0
end

class ARGF_CLASS
  include Enumerable
  def argv
    ARGV
  end
end

ARGF = ARGF_CLASS.new

module RbConfig
  SIZEOF = {"int"=>4, "short"=>2, "long"=>8, "long long"=>8, "__int128"=>16, "off_t"=>8, "void*"=>8, "float"=>4, "double"=>8, "time_t"=>8, "clock_t"=>8, "size_t"=>8, "ptrdiff_t"=>8, "int8_t"=>1, "uint8_t"=>1, "int16_t"=>2, "uint16_t"=>2, "int32_t"=>4, "uint32_t"=>4, "int64_t"=>8, "uint64_t"=>8, "int128_t"=>16, "uint128_t"=>16, "intptr_t"=>8, "uintptr_t"=>8, "ssize_t"=>8, "int_least8_t"=>1, "int_least16_t"=>2, "int_least32_t"=>4, "int_least64_t"=>8, "int_fast8_t"=>1, "int_fast16_t"=>8, "int_fast32_t"=>8, "int_fast64_t"=>8, "intmax_t"=>8, "sig_atomic_t"=>4, "wchar_t"=>4, "wint_t"=>4, "wctrans_t"=>8, "wctype_t"=>8, "_Bool"=>1, "long double"=>16, "float _Complex"=>8, "double _Complex"=>16, "long double _Complex"=>32, "__float128"=>16, "_Decimal32"=>4, "_Decimal64"=>8, "_Decimal128"=>16, "__float80"=>16}
end

class Thread
  CURRENT = {}.compare_by_identity
  def respond_to?(*x)
    false
  end
  def self.current
    CURRENT
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

class SystemExit
end

class Encoding
  UTF_8 = self.new
  US_ASCII = self.new
  IBM437 = self.new
  def self.default_external
    UTF_8
  end
  def self.default_external=(val)
  end
  def self.default_internal
    UTF_8
  end
  def self.default_internal=(val)
  end
end

class Errno < StandardError
  class ENOENT; end;
  class ENOTDIR; end;
  class ENOSYS; end;
  class ENOTSUP; end;
  class EACCES; end;
  class EROFS; end;
end

class Mutex
end

class RangeError < StandardError
end
class FloatDomainError < RangeError
end
class ZeroDivisionError < StandardError
end
class LoadError < StandardError
end
class NameError < StandardError
end


class Module
  def undef_method(sym); end
  def define_method(sym); end
end

RUBY_PLATFORM = "x86_64-linux"
RUBY_VERSION = "3.0.1"
RUBY_ENGINE = "ruruby"
RUBY_DESCRIPTION = "ruruby [x86_64-linux]"
