# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.37 s | 3.26 s | x 2.38 |
| app_mandelbrot.rb | 1.97 s | 8.21 s | x 4.17 |
| fibo.rb | 0.51 s | 2.65 s | x 5.20 |
| block.rb | 0.43 s | 0.93 s | x 2.16 |
| ao_bench.rb | 7.64 s | 33.94 s | x 4.44 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.82  K | 8.22  K | x 0.70 |
| app_mandelbrot.rb | 0.01  M | 1.71  M | x 158.17 |
| fibo.rb | 11.36  K | 2.08  K | x 0.18 |
| block.rb | 10.93  K | 2.03  K | x 0.19 |
| ao_bench.rb | 0.01  M | 4.39  M | x 400.56 |
