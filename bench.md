# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.5

## execution time

|     benchmark     |       ruby       |     ruruby      |  rate  |
| :---------------: | :--------------: | :-------------: | :----: |
| so_mandelbrot.rb  |  1.33 ± 0.03 s   |  2.46 ± 0.09 s  | x 1.85 |
| app_mandelbrot.rb |  1.94 ± 0.02 s   | 10.63 ± 0.09 s  | x 5.48 |
|      fibo.rb      |  0.48 ± 0.00 s   |  2.14 ± 0.04 s  | x 4.43 |
|     block.rb      |  0.40 ± 0.00 s   |  0.88 ± 0.01 s  | x 2.22 |
|    ao_bench.rb    |  7.59 ± 0.05 s   | 39.11 ± 0.16 s  | x 5.15 |
|     optcarrot     | 47.59 ± 1.76 fps | 8.11 ± 0.42 fps | x 5.87 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 11.4K |  2.3K  | x 0.20 |
| app_mandelbrot.rb | 11.3K | 46.3K  | x 4.10 |
|      fibo.rb      | 11.1K |  2.0K  | x 0.18 |
|     block.rb      | 11.2K |  2.0K  | x 0.18 |
|    ao_bench.rb    | 11.4K | 61.1K  | x 5.36 |
|     optcarrot     | 81.6K | 96.4K  | x 1.18 |
