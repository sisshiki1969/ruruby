# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.81 ± 0.01 s | 3.13 ± 0.02 s | x 1.73 |
| app_mandelbrot.rb | 2.23 ± 0.01 s | 7.43 ± 0.06 s | x 3.33 |
| fibo.rb | 0.47 ± 0.00 s | 2.69 ± 0.04 s | x 5.77 |
| block.rb | 0.40 ± 0.00 s | 1.25 ± 0.04 s | x 3.15 |
| ao_bench.rb | 9.28 ± 0.03 s | 29.89 ± 0.35 s | x 3.22 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.8K | 8.9K | x 0.64 |
| app_mandelbrot.rb | 13.9K | 5.2K | x 0.37 |
| fibo.rb | 13.7K | 4.5K | x 0.33 |
| block.rb | 13.6K | 4.5K | x 0.33 |
| ao_bench.rb | 14.4K | 8.6K | x 0.60 |
