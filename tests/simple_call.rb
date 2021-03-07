class C
  def fn
  end
end

o = C.new
1000000.times do
  o.fn()
  o.fn()
  o.fn()
  o.fn()
  o.fn()
  o.fn()
  o.fn()
  o.fn()
  o.fn()
  o.fn()
end