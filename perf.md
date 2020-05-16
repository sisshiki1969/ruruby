# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.33 s | 3.32 s | x 2.50 |
| app_mandelbrot.rb | 1.98 s | 8.35 s | x 4.22 |
| fibo.rb | 0.52 s | 2.64 s | x 5.08 |
| block.rb | 0.45 s | 0.88 s | x 1.96 |
| ao_bench.rb | 7.8 s | 33.78 s | x 4.33 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.18  K | 8.56  K | x 0.77 |
| app_mandelbrot.rb | 0.01  M | 1.78  M | x 150.91 |
| fibo.rb | 10.96  K | 2.06  K | x 0.19 |
| block.rb | 10.98  K | 2.00  K | x 0.18 |
| ao_bench.rb | 0.01  M | 4.57  M | x 393.20 |
