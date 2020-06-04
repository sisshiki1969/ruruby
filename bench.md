# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.41 ± 0.06 s | 2.84 ± 0.17 s | x 2.02 |
| app_mandelbrot.rb | 2.03 ± 0.04 s | 8.97 ± 0.04 s | x 4.42 |
| fibo.rb | 0.51 ± 0.00 s | 2.73 ± 0.07 s | x 5.38 |
| block.rb | 0.44 ± 0.00 s | 0.96 ± 0.03 s | x 2.15 |
| ao_bench.rb | 7.81 ± 0.04 s | 35.38 ± 0.10 s | x 4.53 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.5K | 2.4K | x 0.21 |
| app_mandelbrot.rb | 11.4K | 3.7K | x 0.32 |
| fibo.rb | 11.4K | 2.1K | x 0.18 |
| block.rb | 11.1K | 2.0K | x 0.18 |
| ao_bench.rb | 11.5K | 4.0K | x 0.35 |
