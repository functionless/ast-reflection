; ModuleID = 'probe6.82fa964b-cgu.0'
source_filename = "probe6.82fa964b-cgu.0"
target datalayout = "e-m:e-p:32:32-p10:8:8-p20:8:8-i64:64-n32:64-S128-ni:1:10:20"
target triple = "wasm32-unknown-wasi"

; std::f64::<impl f64>::copysign
; Function Attrs: inlinehint nounwind
define internal double @"_ZN3std3f6421_$LT$impl$u20$f64$GT$8copysign17hcb43f1897879001aE"(double %self, double %sign) unnamed_addr #0 {
start:
  %0 = alloca double, align 8
  %1 = call double @llvm.copysign.f64(double %self, double %sign)
  store double %1, double* %0, align 8
  %2 = load double, double* %0, align 8
  br label %bb1

bb1:                                              ; preds = %start
  ret double %2
}

; probe6::probe
; Function Attrs: nounwind
define hidden void @_ZN6probe65probe17hdc0f2589253af0a7E() unnamed_addr #1 {
start:
; call std::f64::<impl f64>::copysign
  %_1 = call double @"_ZN3std3f6421_$LT$impl$u20$f64$GT$8copysign17hcb43f1897879001aE"(double 1.000000e+00, double -1.000000e+00) #3
  br label %bb1

bb1:                                              ; preds = %start
  ret void
}

; Function Attrs: nofree nosync nounwind readnone speculatable willreturn
declare double @llvm.copysign.f64(double, double) #2

attributes #0 = { inlinehint nounwind "target-cpu"="generic" }
attributes #1 = { nounwind "target-cpu"="generic" }
attributes #2 = { nofree nosync nounwind readnone speculatable willreturn }
attributes #3 = { nounwind }
