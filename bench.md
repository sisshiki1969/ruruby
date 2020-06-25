# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.73 ± 0.03 s | 2.43 ± 0.02 s  | x 1.40 |
| app_mandelbrot.rb | 2.20 ± 0.04 s | 5.78 ± 0.02 s  | x 2.63 |
|      fibo.rb      | 0.45 ± 0.01 s | 1.92 ± 0.03 s  | x 4.28 |
|     block.rb      | 0.37 ± 0.01 s | 0.99 ± 0.01 s  | x 2.68 |
|    ao_bench.rb    | 9.04 ± 0.07 s | 25.61 ± 0.16 s | x 2.83 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 49.52 ± 1.17 fps  | 9.36 ± 0.07 fps  | x 5.29 |
| optcarrot --opt | 149.45 ± 4.84 fps | 38.59 ± 0.63 fps | x 3.87 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  5.1K  | x 0.36  |
| app_mandelbrot.rb | 14.0K |  6.7K  | x 0.48  |
|      fibo.rb      | 13.7K |  4.9K  | x 0.35  |
|     block.rb      | 13.7K |  4.8K  | x 0.35  |
|    ao_bench.rb    | 14.5K |  7.5K  | x 0.52  |
|     optcarrot     | 68.4K | 81.5K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.7M  | x 19.52 |
