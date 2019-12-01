f0 = ->{100}
f1 = ->x{x*6}
f2 = ->(x,y){x*y}
puts f0.call
puts f1.call(50)    # puts f1.call 50 => error
puts f2.call(5,7)   # puts f2.call 5,7 => error