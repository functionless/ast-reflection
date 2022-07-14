let i;

// arrow expression
{
  const capture_default_reference_to_param_in_closure = (
    a = () => {
      i; // i#1
    }
  ) => {};
  
  const dont_capture_let_shadowed_by_param = (i) => {
    i;
  };
  
  const dont_capture_default_reference_to_param = (i, a = i) => {};
  
  const dont_capture_default_reference_to_param_in_closure = (
    i,
    a = () => {
      i;
    }
  ) => {};
  
  const arg_destructuring_test = (
    i,
    { a, b: c, d: [e], ...f },
    [g, { h, i: j }, [k], ...l],
    ...m
  ) => {
    return () => {
      i;
      a;
      // b;
      c;
      // d;
      e;
      f;
      g;
      h;
      // i;
      j;
      k;
      l;
      m;
    };
  };
}

// function expressions
{
  const capture_default_reference_to_param_in_closure = function (
    a = function () {
      i; // i#1
    }
  ) {};
  
  const dont_capture_let_shadowed_by_param = function (i) {
    i;
  };
  
  const dont_capture_default_reference_to_param = function (i, a = i) {};
  
  const dont_capture_default_reference_to_param_in_closure = function (
    i,
    a = function () {
      i;
    }
  ) {};
  
  const arg_destructuring_test = function (
    i,
    { a, b: c, d: [e], ...f },
    [g, { h, i: j }, [k], ...l],
    ...m
  ) {
    return function () {
      i;
      a;
      // b;
      c;
      // d;
      e;
      f;
      g;
      h;
      // i;
      j;
      k;
      l;
      m;
    };
  };
}

// function declarations
{
  function capture_default_reference_to_param_in_closure (
    a = () => {
      i; // i#1
    }
  ) {};
  
  function dont_capture_let_shadowed_by_param (i) {
    i;
  };
  
  function dont_capture_default_reference_to_param (i, a = i) {};
  
  function dont_capture_default_reference_to_param_in_closure (
    i,
    a = () => {
      i;
    }
  ) {};
  
  function arg_destructuring_test (
    i,
    { a, b: c, d: [e], ...f },
    [g, { h, i: j }, [k], ...l],
    ...m
  ) {
    return () => {
      i;
      a;
      // b;
      c;
      // d;
      e;
      f;
      g;
      h;
      // i;
      j;
      k;
      l;
      m;
    };
  };
}