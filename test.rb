a = 100
p = ->{->{puts a}}
p.call.call
a = 200
p.call.call