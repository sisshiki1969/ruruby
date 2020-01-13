def fn(a,b,c,d,e=100,f=77,*g,h,i,&j)
    puts "a= #{a}"
    puts "b= #{b}"
    puts "c= #{c}"
    puts "d= #{d}"
    puts "e= #{e}"
    puts "f= #{f}"
    puts "g= #{g}"
    puts "h= #{h}"
    puts "i= #{i}"
    puts "j= #{j}"
    j.call if block_given?
end

fn 1,2,3,4,5,6,7,8,9,10 do
    puts "block"
end

fn 1,2,3,4,5,6
