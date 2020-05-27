# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.37 s | 2.99 s | x 2.18 |
| app_mandelbrot.rb | 1.97 s | 8.85 s | x 4.49 |
| fibo.rb | 0.5 s | 2.68 s | x 5.36 |
| block.rb | 0.43 s | 1.0 s | x 2.33 |
| ao_bench.rb | 7.72 s | 35.73 s | x 4.63 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.42  K | 5.76  K | x 0.50 |
| app_mandelbrot.rb | 11.53  K | 31.08  K | x 2.69 |
| fibo.rb | 10.97  K | 2.10  K | x 0.19 |
| block.rb | 11.24  K | 2.06  K | x 0.18 |
| ao_bench.rb | 11.29  K | 54.03  K | x 4.78 |
