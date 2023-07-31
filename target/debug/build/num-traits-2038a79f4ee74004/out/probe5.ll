; ModuleID = 'probe5.232ef089577d744e-cgu.0'
source_filename = "probe5.232ef089577d744e-cgu.0"
target datalayout = "e-m:o-i64:64-i128:128-n32:64-S128"
target triple = "arm64-apple-macosx11.0.0"

@alloc_eff8ba1c8a4815b79643d4cacf29282e = private unnamed_addr constant <{ [75 x i8] }> <{ [75 x i8] c"/rustc/32303b219d4dffa447aa606bc11c7a648f44a862/library/core/src/num/mod.rs" }>, align 1
@alloc_d0936921a6643ade0e5db0608a1e829b = private unnamed_addr constant <{ ptr, [16 x i8] }> <{ ptr @alloc_eff8ba1c8a4815b79643d4cacf29282e, [16 x i8] c"K\00\00\00\00\00\00\00w\04\00\00\05\00\00\00" }>, align 8
@str.0 = internal constant [25 x i8] c"attempt to divide by zero"

; probe5::probe
; Function Attrs: uwtable
define void @_ZN6probe55probe17h6d0fa74431ec83f2E() unnamed_addr #0 {
start:
  %0 = call i1 @llvm.expect.i1(i1 false, i1 false)
  br i1 %0, label %panic.i, label %"_ZN4core3num21_$LT$impl$u20$u32$GT$10div_euclid17h577a331747b30441E.exit"

panic.i:                                          ; preds = %start
; call core::panicking::panic
  call void @_ZN4core9panicking5panic17hdf341f202c3d5554E(ptr align 1 @str.0, i64 25, ptr align 8 @alloc_d0936921a6643ade0e5db0608a1e829b) #3
  unreachable

"_ZN4core3num21_$LT$impl$u20$u32$GT$10div_euclid17h577a331747b30441E.exit": ; preds = %start
  ret void
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(none)
declare i1 @llvm.expect.i1(i1, i1) #1

; core::panicking::panic
; Function Attrs: cold noinline noreturn uwtable
declare void @_ZN4core9panicking5panic17hdf341f202c3d5554E(ptr align 1, i64, ptr align 8) unnamed_addr #2

attributes #0 = { uwtable "frame-pointer"="non-leaf" "target-cpu"="apple-m1" }
attributes #1 = { nocallback nofree nosync nounwind willreturn memory(none) }
attributes #2 = { cold noinline noreturn uwtable "frame-pointer"="non-leaf" "target-cpu"="apple-m1" }
attributes #3 = { noreturn }

!llvm.module.flags = !{!0}
!llvm.ident = !{!1}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{!"rustc version 1.73.0-nightly (32303b219 2023-07-29)"}
