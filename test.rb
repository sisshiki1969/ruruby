class C; end
D = 0
C.class_eval "def fn; 77; end; D = 1"
puts C.new.fn #77
puts C::D #1
puts D #0
C.class_eval do
  def gn
    99
  end
  D = 2
end
puts C.new.gn #99
puts C::D #1
puts D #2
