yarn link

ROOT_DIR=$(pwd)
TEST_DIR="${ROOT_DIR}/.test"

mkdir -p ${TEST_DIR}

clean_up() {
  rm -rf ${TEST_DIR}
}

trap clean_up EXIT

cd .test

git clone --depth 1 https://github.com/functionless/functionless.git

cd functionless

yarn
yarn link @functionless/ast-reflection
yarn compile
yarn test
