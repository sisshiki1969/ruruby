class A
  def fn
    f
  end
  def gn
    fn
  end
end

d = A.new

d.gn
