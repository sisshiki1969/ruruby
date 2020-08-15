P=Struct.new(:x,:d,:p,:v);
Z=?\s
S=Z*4
M=(-5**7..b=0).map{[]};
A=s=[];
t=Time.now;
q=?y.succ;
(
   s=S.scan(/.+/);
   M[0] << P[25i-b%3*5i-9,0,0 ,2+1i];
   60.times{|i|
      j=i%20;
      i<40 ? [M[j -1], m=M[j], M[j+1]].each{|n|
         m.each{|p|
            n.each{|q|
               d=p.x-q.x;
               w=d.abs-4;
               w<0 && (i<20? p.d+=w*w : p.p+=w * (d * (3-p.d-q.d) + (p.v-q.v)*4) / p.d)
            }
         }
      } : M.shift.each {|p|
         puts "p = #{ p.inspect }"
         y,x= (p.x += p.v += p.p/10).rect;
         p.p= [43- b/9.0-y, 1].min - [x,p.d=0, x-92].sort[1]*2i;
         p.v/= [1, p.v.abs/2].max;
         M[20-j+[0, (x+4).div(5), 19].sort[1]] << p;
         35.times{|w|
            v=x.to_i-3+w%7;
            puts ".. = #{ y.div(2)-2+w/7 }"
            puts "s = #{ s.inspect }"
            c=s[w=y.div(2)-2+w/7];
            (x-v)**2+(y-w*2)**2<16 && 0<=w && c && (k=(w*2-21)**2/99) <= v && c[v] && k+79 != v && c[v]=q
         }
      }
   };
)
