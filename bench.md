# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.91 ± 0.17 s | 2.62 ± 0.02 s | x 1.37 |
| app_mandelbrot.rb | 2.23 ± 0.02 s | 6.39 ± 0.18 s | x 2.86 |
| fibo.rb | 0.47 ± 0.01 s | 2.06 ± 0.00 s | x 4.39 |
| block.rb | 0.40 ± 0.01 s | 1.08 ± 0.01 s | x 2.71 |
| ao_bench.rb | 9.43 ± 0.25 s | 26.29 ± 0.24 s | x 2.79 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.9K | 4.7K | x 0.34 |
| app_mandelbrot.rb | 13.9K | 20.9K | x 1.51 |
| fibo.rb | 13.6K | 4.6K | x 0.33 |
| block.rb | 13.6K | 4.6K | x 0.34 |
| ao_bench.rb | 14.4K | 20.1K | x 1.40 |
