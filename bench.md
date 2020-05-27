# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4

## execution time

|     benchmark     |  ruby  | ruruby  |  rate  |
| :---------------: | :----: | :-----: | :----: |
| so_mandelbrot.rb  | 1.33 s | 2.98 s  | x 2.24 |
| app_mandelbrot.rb | 1.9 s  | 8.97 s  | x 4.72 |
|      fibo.rb      | 0.51 s | 2.72 s  | x 5.33 |
|     block.rb      | 0.44 s | 1.06 s  | x 2.41 |
|    ao_bench.rb    | 7.81 s | 35.16 s | x 4.50 |

## memory consumption

|     benchmark     |  ruby   | ruruby  |  rate  |
| :---------------: | :-----: | :-----: | :----: |
| so_mandelbrot.rb  | 11.84 K | 5.76 K  | x 0.49 |
| app_mandelbrot.rb | 11.57 K | 32.54 K | x 2.81 |
|      fibo.rb      | 10.92 K | 2.11 K  | x 0.19 |
|     block.rb      | 11.24 K | 2.06 K  | x 0.18 |
|    ao_bench.rb    | 11.49 K | 57.19 K | x 4.98 |
