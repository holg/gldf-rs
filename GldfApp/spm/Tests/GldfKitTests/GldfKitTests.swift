import XCTest
@testable import GldfKit

final class GldfKitTests: XCTestCase {
    func testLibraryVersion() throws {
        let version = gldfLibraryVersion()
        print("GLDF Library version: \(version)")
        XCTAssertFalse(version.isEmpty, "Version should not be empty")
    }
}
