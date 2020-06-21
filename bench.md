# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.92 ± 0.03 s | 2.32 ± 0.02 s  | x 1.21 |
| app_mandelbrot.rb | 2.34 ± 0.02 s | 6.26 ± 0.12 s  | x 2.68 |
|      fibo.rb      | 0.50 ± 0.02 s | 1.95 ± 0.01 s  | x 3.87 |
|     block.rb      | 0.39 ± 0.01 s | 1.06 ± 0.04 s  | x 2.71 |
|    ao_bench.rb    | 9.81 ± 0.30 s | 26.44 ± 0.27 s | x 2.70 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 45.49 ± 1.86 fps  | 7.91 ± 0.08 fps  | x 5.75 |
| optcarrot --opt | 136.92 ± 5.80 fps | 35.99 ± 0.87 fps | x 3.80 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  4.8K  | x 0.34  |
| app_mandelbrot.rb | 14.0K |  6.4K  | x 0.46  |
|      fibo.rb      | 13.8K |  4.6K  | x 0.34  |
|     block.rb      | 13.8K |  4.5K  | x 0.33  |
|    ao_bench.rb    | 14.5K |  7.2K  | x 0.50  |
|     optcarrot     | 68.5K | 81.2K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 16.94 |
