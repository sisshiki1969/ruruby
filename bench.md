# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 2.54 ± 0.02 s | 2.94 ± 0.23 s | x 1.16 |
| app_mandelbrot.rb | 2.07 ± 0.02 s | 9.50 ± 0.10 s | x 4.58 |
| fibo.rb | 0.53 ± 0.01 s | 2.99 ± 0.11 s | x 5.65 |
| block.rb | 0.46 ± 0.00 s | 1.05 ± 0.03 s | x 2.29 |
| ao_bench.rb | 8.30 ± 0.14 s | 37.71 ± 0.18 s | x 4.54 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.9K | 5.8K | x 0.49 |
| app_mandelbrot.rb | 11.8K | 32.1K | x 2.71 |
| fibo.rb | 11.5K | 2.1K | x 0.18 |
| block.rb | 11.7K | 2.1K | x 0.18 |
| ao_bench.rb | 11.7K | 56.7K | x 4.84 |
