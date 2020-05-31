# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.35 ± 0.01 s | 2.74 ± 0.10 s | x 2.02 |
| app_mandelbrot.rb | 2.08 ± 0.10 s | 8.91 ± 0.05 s | x 4.28 |
| fibo.rb | 0.54 ± 0.02 s | 2.77 ± 0.02 s | x 5.14 |
| block.rb | 0.47 ± 0.01 s | 0.99 ± 0.01 s | x 2.13 |
| ao_bench.rb | 8.45 ± 0.22 s | 32.46 ± 8.14 s | x 3.84 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.3K | 5.7K | x 0.51 |
| app_mandelbrot.rb | 0.0M | 1.8M | x 154.07 |
| fibo.rb | 11.7K | 2.1K | x 0.18 |
| block.rb | 11.8K | 2.0K | x 0.17 |
| ao_bench.rb | 0.0M | 4.0M | x 333.64 |
